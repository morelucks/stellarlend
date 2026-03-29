use soroban_sdk::{contract, contractimpl, Address, Env, Map, Symbol, Vec, String};

pub mod admin;
pub mod analytics;
pub mod borrow;
pub mod bridge;
pub mod config;
pub mod cross_asset;
pub mod deposit;
pub mod errors;
pub mod events;
pub mod flash_loan;
pub mod governance;
pub mod interest_rate;
pub mod liquidate;
pub mod multisig;
pub mod oracle;
pub mod recovery;
pub mod reentrancy;
pub mod repay;
pub mod reserve;
pub mod risk_management;
pub mod risk_params;
pub mod storage;
pub mod types;
pub mod withdraw;

#[cfg(test)]
// mod tests;

use crate::deposit::DepositDataKey;
use crate::risk_management::RiskManagementError;
use crate::interest_rate::InterestRateError;

/// Helper function to require admin authorization
fn require_admin(env: &Env, caller: &Address) -> Result<(), RiskManagementError> {
    caller.require_auth();
    let admin_key = DepositDataKey::Admin;
    let admin = env
        .storage()
        .persistent()
        .get::<DepositDataKey, Address>(&admin_key)
        .ok_or(RiskManagementError::Unauthorized)?;

    if caller != &admin {
        return Err(RiskManagementError::Unauthorized);
    }
    Ok(())
}

/// The StellarLend core contract.
#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env) -> String {
        String::from_str(&env, "Hello")
    }

    pub fn initialize(env: Env, admin: Address) -> Result<(), RiskManagementError> {
        if crate::admin::has_admin(&env) {
            return Err(RiskManagementError::Unauthorized);
        }
        crate::admin::set_admin(&env, admin.clone(), None)
            .map_err(|_| RiskManagementError::Unauthorized)?;
        risk_management::initialize_risk_management(&env, admin.clone())?;
        risk_params::initialize_risk_params(&env).map_err(|_| RiskManagementError::InvalidParameter)?;
        interest_rate::initialize_interest_rate_config(&env, admin).map_err(|e| {
            if e == InterestRateError::AlreadyInitialized {
                RiskManagementError::AlreadyInitialized
            } else {
                RiskManagementError::Unauthorized
            }
        })?;
        Ok(())
    }

    pub fn transfer_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), admin::AdminError> {
        admin::set_admin(&env, new_admin, Some(caller))
    }

    pub fn deposit_collateral(env: Env, user: Address, asset: Option<Address>, amount: i128) -> Result<i128, deposit::DepositError> {
        deposit::deposit_collateral(&env, user, asset, amount)
    }

    pub fn set_risk_params(
        env: Env,
        caller: Address,
        min_collateral_ratio: Option<i128>,
        liquidation_threshold: Option<i128>,
        close_factor: Option<i128>,
        liquidation_incentive: Option<i128>,
    ) -> Result<(), RiskManagementError> {
        require_admin(&env, &caller)?;
        risk_params::set_risk_params(&env, min_collateral_ratio, liquidation_threshold, close_factor, liquidation_incentive)
            .map_err(|_| RiskManagementError::InvalidParameter)
    }

    pub fn borrow_asset(env: Env, user: Address, asset: Option<Address>, amount: i128) -> Result<i128, borrow::BorrowError> {
        borrow::borrow_asset(&env, user, asset, amount)
    }

    pub fn repay_debt(env: Env, user: Address, asset: Option<Address>, amount: i128) -> Result<(i128, i128, i128), repay::RepayError> {
        repay::repay_debt(&env, user, asset, amount)
    }

    pub fn withdraw_collateral(env: Env, user: Address, asset: Option<Address>, amount: i128) -> Result<i128, withdraw::WithdrawError> {
        withdraw::withdraw_collateral(&env, user, asset, amount)
    }

    pub fn liquidate(env: Env, caller: Address, paused: bool) -> Result<(), RiskManagementError> {
        risk_management::set_emergency_pause(&env, caller, paused)
    }

    pub fn execute_flash_loan(env: Env, user: Address, asset: Address, amount: i128, callback: Address) -> Result<i128, flash_loan::FlashLoanError> {
        flash_loan::execute_flash_loan(&env, user, asset, amount, callback)
    }

    pub fn repay_flash_loan(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), flash_loan::FlashLoanError> {
        flash_loan::repay_flash_loan(&env, user, asset, amount)
    }

    pub fn can_be_liquidated(env: Env, collateral_value: i128, debt_value: i128) -> Result<bool, risk_params::RiskParamsError> {
        risk_params::can_be_liquidated(&env, collateral_value, debt_value)
    }

    pub fn get_max_liquidatable_amount(env: Env, debt_value: i128) -> Result<i128, risk_params::RiskParamsError> {
        risk_params::get_max_liquidatable_amount(&env, debt_value)
    }

    pub fn get_liquidation_incentive_amount(env: Env, liquidated_amount: i128) -> Result<i128, risk_params::RiskParamsError> {
        risk_params::get_liquidation_incentive_amount(&env, liquidated_amount)
    }

    pub fn require_min_collateral_ratio(env: Env, collateral_value: i128, debt_value: i128) -> Result<(), risk_params::RiskParamsError> {
        risk_params::require_min_collateral_ratio(&env, collateral_value, debt_value)
    }
}

#[cfg(test)]
mod test_reentrancy;
#[cfg(test)]
mod test_zero_amount;
#[cfg(test)]
mod flash_loan_test;
