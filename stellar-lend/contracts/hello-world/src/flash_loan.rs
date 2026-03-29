//! # Flash Loan Module
//!
//! Provides uncollateralized flash loan functionality for the lending protocol.
//!
//! Flash loans allow users to borrow assets without collateral, provided the loan
//! (principal + fee) is repaid within the same transaction via a callback contract.
//!
//! ## Fee Structure
//! - Default fee: 9 basis points (0.09%) of the borrowed amount.
//! - Fee is configurable by the admin.
//!
//! ## Reentrancy Protection
//! An active flash loan is recorded per (user, asset) pair. A second flash loan
//! for the same pair is rejected until the first is repaid, preventing reentrancy.
//! This implementation uses a RAII guard to ensure the guard is always cleared,
//! but only after all verification steps are completed.
//!
//! ## Invariants
//! - The borrowed amount must be within configured min/max limits.
//! - The contract must have sufficient liquidity to fund the loan.
//! - Repayment must cover principal + fee in full.

#![allow(unused)]
use crate::events::{
    emit_flash_loan_initiated, emit_flash_loan_repaid, FlashLoanInitiatedEvent,
    FlashLoanRepaidEvent,
};
use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal, Map, Symbol, Val, Vec};

use crate::deposit::DepositDataKey;

/// Errors that can occur during flash loan operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FlashLoanError {
    /// Flash loan amount must be greater than zero
    InvalidAmount = 1,
    /// Asset address is invalid
    InvalidAsset = 2,
    /// Insufficient liquidity for flash loan
    InsufficientLiquidity = 3,
    /// Flash loan operations are currently paused
    FlashLoanPaused = 4,
    /// Flash loan not repaid within transaction
    NotRepaid = 5,
    /// Insufficient repayment amount
    InsufficientRepayment = 6,
    /// Overflow occurred during calculation
    Overflow = 7,
    /// Reentrancy detected
    Reentrancy = 8,
    /// Invalid callback
    InvalidCallback = 9,
    /// Callback execution failed
    CallbackFailed = 10,
}

/// Storage keys for flash loan-related data
#[contracttype]
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum FlashLoanDataKey {
    /// Global flash loan configuration (fee, limits)
    FlashLoanConfig,
    /// Active flash loan record for repayment tracking
    /// Key: (User Address, Asset Address)
    ActiveFlashLoan(Address, Address),
    /// Reentrancy guard lock key for flash loans
    /// Key: (User Address, Asset Address)
    FlashLoanGuard(Address, Address),
    /// Pause switches for flash loan operations
    PauseSwitches,
}

/// Configuration for flash loan operations
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanConfig {
    /// Flash loan fee in basis points (e.g., 9 = 0.09%)
    pub fee_bps: i128,
    /// Maximum allowed flash loan amount for any single asset
    pub max_amount: i128,
    /// Minimum allowed flash loan amount
    pub min_amount: i128,
}

/// Record of an active flash loan
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanRecord {
    /// Amount borrowed
    pub amount: i128,
    /// Fee to be repaid
    pub fee: i128,
    /// Timestamp when loan was initiated
    pub timestamp: u64,
    /// Callback contract address
    pub callback: Address,
}

const DEFAULT_FLASH_LOAN_FEE_BPS: i128 = 9;
const DEFAULT_MAX_FLASH_LOAN_AMOUNT: i128 = 1_000_000_000_000; // Example: 1M tokens
const DEFAULT_MIN_FLASH_LOAN_AMOUNT: i128 = 100; // Example: 100 tokens

/// Default configuration
fn get_default_config() -> FlashLoanConfig {
    FlashLoanConfig {
        fee_bps: DEFAULT_FLASH_LOAN_FEE_BPS,
        max_amount: DEFAULT_MAX_FLASH_LOAN_AMOUNT,
        min_amount: DEFAULT_MIN_FLASH_LOAN_AMOUNT,
    }
}

/// Get flash loan configuration
fn get_flash_loan_config(env: &Env) -> FlashLoanConfig {
    let config_key = FlashLoanDataKey::FlashLoanConfig;
    env.storage()
        .persistent()
        .get::<FlashLoanDataKey, FlashLoanConfig>(&config_key)
        .unwrap_or_else(get_default_config)
}

/// Calculate flash loan fee
fn calculate_flash_loan_fee(env: &Env, amount: i128) -> Result<i128, FlashLoanError> {
    let config = get_flash_loan_config(env);

    // Fee = amount * fee_bps / 10000
    amount
        .checked_mul(config.fee_bps)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(10000)
        .ok_or(FlashLoanError::Overflow)
}

/// Check if flash loan is active
fn is_flash_loan_active(env: &Env, user: &Address, asset: &Address) -> bool {
    let loan_key: soroban_sdk::Val = FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    env.storage().temporary().has(&loan_key)
}

/// Record flash loan details
fn record_flash_loan(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
    fee: i128,
    callback: &Address,
) {
    let loan_key: soroban_sdk::Val = FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    let record = FlashLoanRecord {
        amount,
        fee,
        timestamp: env.ledger().timestamp(),
        callback: callback.clone(),
    };
    env.storage().temporary().set(&loan_key, &record);
}

/// Clear flash loan record
fn clear_flash_loan(env: &Env, user: &Address, asset: &Address) {
    let loan_key: soroban_sdk::Val = FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    env.storage().temporary().remove(&loan_key);
}

/// Execute flash loan
///
/// Refactored to use a RAII guard and unified callback pattern.
/// The reentrancy guard is cleared only after successful repayment verification.
pub fn execute_flash_loan(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    callback: Address,
) -> Result<i128, FlashLoanError> {
    // 1. Validation
    if amount <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    let config = get_flash_loan_config(env);
    if amount < config.min_amount || amount > config.max_amount {
        return Err(FlashLoanError::InvalidAmount);
    }

    let pause_map_key = FlashLoanDataKey::PauseSwitches;
    if let Some(pause_map) = env.storage().persistent().get::<FlashLoanDataKey, Map<Symbol, bool>>(&pause_map_key) {
        if pause_map.get(Symbol::new(env, "pause_flash_loan")).unwrap_or(false) {
            return Err(FlashLoanError::FlashLoanPaused);
        }
    }

    // 2. Preparation
    let fee = calculate_flash_loan_fee(env, amount)?;
    let total_required = amount.checked_add(fee).ok_or(FlashLoanError::Overflow)?;

    let token_client = soroban_sdk::token::Client::new(env, &asset);
    let initial_balance = token_client.balance(&env.current_contract_address());
    if initial_balance < amount {
        return Err(FlashLoanError::InsufficientLiquidity);
    }

    // 3. Initiate Guards (RAII)
    // The granular guard automatically clears when execute_flash_loan finishes.
    
    // Granular guard prevents re-entry into flash loan for same user/asset
    // Note: We intentionally do NOT use a global guard here because the callback
    // MUST be allowed to call back into the protocol (e.g., to repay the loan).
    let lock_key: soroban_sdk::Val = FlashLoanDataKey::FlashLoanGuard(user.clone(), asset.clone()).into_val(env);
    let _granular_guard = crate::reentrancy::ReentrancyGuard::new_with_key(env, lock_key)
        .map_err(|_| FlashLoanError::Reentrancy)?;

    // Record the loan details for repay_flash_loan helper
    record_flash_loan(env, &user, &asset, amount, fee, &callback);

    // 4. Transfer funds to user
    token_client.transfer(&env.current_contract_address(), &callback, &amount);

    emit_flash_loan_initiated(
        env,
        FlashLoanInitiatedEvent {
            user: user.clone(),
            asset: asset.clone(),
            amount,
            fee,
            callback: callback.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    // 5. Invoke Callback
    let callback_symbol = Symbol::new(env, "on_flash_loan");
    let _: soroban_sdk::Val = env.invoke_contract(
        &callback,
        &callback_symbol,
        (user.clone(), asset.clone(), amount, fee).into_val(env),
    );

    // 6. Repayment via Transfer From
    // Soroban blocks re-entry from the callback, so the callback cannot call `repay_flash_loan`.
    // Instead, the callback must authorize the lending contract to pull the funds
    // (principal + fee), and we execute the pull here after the callback returns.
    token_client.transfer_from(
        &env.current_contract_address(),
        &callback,
        &env.current_contract_address(),
        &total_required,
    );

    // 7. Credit fee to reserve
    if fee > 0 {
        let reserve_key = DepositDataKey::ProtocolReserve(Some(asset.clone()));
        let current_reserve = env.storage().persistent().get::<DepositDataKey, i128>(&reserve_key).unwrap_or(0);
        let new_reserve = current_reserve.checked_add(fee).ok_or(FlashLoanError::Overflow)?;
        env.storage().persistent().set(&reserve_key, &new_reserve);
    }

    // Explicitly clear the record if successfully finished (optional, but cleaner)
    // The guards will still drop and do their job.
    clear_flash_loan(env, &user, &asset);

    Ok(total_required)
}

/// Repay flash loan (Helper)
///
/// Can be called by the user/callback to repay the loan.
/// Does NOT clear the guard; clearing is handled by the execute_flash_loan guard.
pub fn repay_flash_loan(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<(), FlashLoanError> {
    let loan_key: soroban_sdk::Val = FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    let record = env.storage().temporary().get::<Val, FlashLoanRecord>(&loan_key)
        .ok_or(FlashLoanError::NotRepaid)?;

    let total_required = record.amount.checked_add(record.fee).ok_or(FlashLoanError::Overflow)?;
    if amount < total_required {
        return Err(FlashLoanError::InsufficientRepayment);
    }

    let token_client = soroban_sdk::token::Client::new(env, &asset);
    
    // Transfer funds from user back to contract
    token_client.transfer_from(
        &env.current_contract_address(),
        &user,
        &env.current_contract_address(),
        &total_required,
    );

    emit_flash_loan_repaid(
        env,
        FlashLoanRepaidEvent {
            user: user.clone(),
            asset: asset.clone(),
            amount: record.amount,
            fee: record.fee,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}

/// Set flash loan configuration (Admin only)
pub fn set_flash_loan_config(
    env: &Env,
    caller: Address,
    new_config: FlashLoanConfig,
) -> Result<(), FlashLoanError> {
    crate::admin::require_admin(env, &caller).map_err(|_| FlashLoanError::InvalidCallback)?;

    if !(0..=10000).contains(&new_config.fee_bps) {
        return Err(FlashLoanError::InvalidAmount);
    }

    let config_key = FlashLoanDataKey::FlashLoanConfig;
    env.storage().persistent().set(&config_key, &new_config);
    Ok(())
}

/// Set flash loan fee separately (Admin only)
pub fn set_flash_loan_fee(env: &Env, caller: Address, fee_bps: i128) -> Result<(), FlashLoanError> {
    crate::admin::require_admin(env, &caller).map_err(|_| FlashLoanError::InvalidCallback)?;

    if !(0..=10000).contains(&fee_bps) {
        return Err(FlashLoanError::InvalidAmount);
    }

    let mut config = get_flash_loan_config(env);
    config.fee_bps = fee_bps;
    
    let config_key = FlashLoanDataKey::FlashLoanConfig;
    env.storage().persistent().set(&config_key, &config);
    Ok(())
}
