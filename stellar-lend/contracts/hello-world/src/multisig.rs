use soroban_sdk::{Address, Env, Vec};

use crate::governance::{
    approve_proposal, execute_proposal, get_multisig_config, set_multisig_config,
    get_proposal, get_proposal_approvals,
};
use crate::errors::GovernanceError;
use crate::types::{MultisigConfig, Proposal, ProposalStatus, ProposalType};

pub fn ms_set_admins(
    env: &Env,
    caller: Address,
    admins: Vec<Address>,
    threshold: u32,
) -> Result<(), GovernanceError> {
    if admins.is_empty() || threshold == 0 || threshold > admins.len() {
        return Err(GovernanceError::InvalidMultisigConfig);
    }

    // Duplicate check
    for i in 0..admins.len() {
        for j in (i + 1)..admins.len() {
            if admins.get(i).unwrap() == admins.get(j).unwrap() {
                return Err(GovernanceError::InvalidMultisigConfig);
            }
        }
    }

    let existing = get_multisig_config(env);
    if existing.is_none() {
        // Bootstrap
        let config = MultisigConfig { admins, threshold };
        set_multisig_config(env, caller, config.admins, config.threshold)?;
        Ok(())
    } else {
        // Post-bootstrap
        set_multisig_config(env, caller, admins, threshold)
    }
}

pub fn ms_propose_set_min_cr(
    env: &Env,
    proposer: Address,
    new_ratio: i128,
) -> Result<u64, GovernanceError> {
    if new_ratio <= 10_000 {
        return Err(GovernanceError::InvalidProposal);
    }

    // Delegates to governance.rs using a generic proposal type
    let proposal_id = crate::governance::create_proposal(
        env,
        proposer.clone(),
        ProposalType::MinCollateralRatio(new_ratio),
        soroban_sdk::String::from_str(env, "Adjust minimum collateral ratio"),
        None,
    )?;

    // Proposer auto-approves
    approve_proposal(env, proposer, proposal_id)?;

    Ok(proposal_id)
}

pub fn ms_approve(
    env: &Env,
    approver: Address,
    proposal_id: u64,
) -> Result<(), GovernanceError> {
    approve_proposal(env, approver, proposal_id)
}

pub fn ms_execute(
    env: &Env,
    executor: Address,
    proposal_id: u64,
) -> Result<(), GovernanceError> {
    execute_proposal(env, executor, proposal_id)
}

pub fn get_ms_admins(env: &Env) -> Option<Vec<Address>> {
    get_multisig_config(env).map(|c| c.admins)
}

pub fn get_ms_threshold(env: &Env) -> u32 {
    get_multisig_config(env).map(|c| c.threshold).unwrap_or(1)
}

pub fn get_ms_proposal(env: &Env, proposal_id: u64) -> Option<Proposal> {
    get_proposal(env, proposal_id)
}

pub fn get_ms_approvals(env: &Env, proposal_id: u64) -> Option<Vec<Address>> {
    get_proposal_approvals(env, proposal_id)
}