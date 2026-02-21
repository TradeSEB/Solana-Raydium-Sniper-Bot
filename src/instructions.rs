use anyhow::{Context, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use std::str::FromStr;

use crate::config::{RAYDIUM_AMM_V4_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID};

/// Raydium instruction discriminators
/// 
/// Note: These are approximate. In production, you should:
/// 1. Use the actual Raydium IDL
/// 2. Or parse instruction data to extract discriminators
/// 3. Or use anchor-client to build instructions
pub mod discriminators {
    /// Initialize2 instruction (AMM v4)
    pub const INITIALIZE2: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    
    /// Initialize instruction (AMM v4)
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    
    /// Swap instruction (AMM v4)
    pub const SWAP: [u8; 8] = [225, 226, 218, 232, 240, 105, 206, 129];
    
    /// CPMM Initialize
    pub const CPMM_INITIALIZE: [u8; 8] = [0; 8]; // Placeholder - verify with IDL
    
    /// CPMM Swap
    pub const CPMM_SWAP: [u8; 8] = [0; 8]; // Placeholder - verify with IDL
}

/// Build a Raydium AMM v4 swap instruction
/// 
/// This is a simplified version. In production, you should:
/// 1. Use the Raydium SDK or Anchor IDL
/// 2. Properly derive all required accounts (pool, amm, authority, etc.)
/// 3. Calculate amount_in and min_amount_out with slippage
pub fn build_raydium_swap_instruction(
    user: &Pubkey,
    pool: &Pubkey,
    amm: &Pubkey,
    amm_authority: &Pubkey,
    amm_open_orders: &Pubkey,
    amm_target_orders: &Pubkey,
    pool_coin_token_account: &Pubkey,
    pool_pc_token_account: &Pubkey,
    serum_program_id: &Pubkey,
    serum_market: &Pubkey,
    user_source_token_account: &Pubkey,
    user_dest_token_account: &Pubkey,
    user_source_owner: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM_ID)
        .context("Failed to parse Raydium AMM v4 program ID")?;

    // Build instruction data: discriminator + amount_in + min_amount_out
    let mut data = Vec::new();
    data.extend_from_slice(&discriminators::SWAP);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());

    // Account metas (order matters - verify with IDL)
    // This is simplified - actual instruction requires many more accounts
    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new_readonly(*amm, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_open_orders, false),
        AccountMeta::new(*amm_target_orders, false),
        AccountMeta::new(*pool_coin_token_account, false),
        AccountMeta::new(*pool_pc_token_account, false),
        AccountMeta::new(*user_source_token_account, false),
        AccountMeta::new(*user_dest_token_account, false),
        AccountMeta::new_readonly(*serum_program_id, false),
        AccountMeta::new_readonly(*serum_market, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Build a Raydium CPMM swap instruction
/// 
/// CPMM uses a simpler constant product formula
pub fn build_cpmm_swap_instruction(
    user: &Pubkey,
    pool: &Pubkey,
    user_source_token_account: &Pubkey,
    user_dest_token_account: &Pubkey,
    pool_source_token_account: &Pubkey,
    pool_dest_token_account: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)
        .context("Failed to parse Raydium CPMM program ID")?;

    // Build instruction data
    let mut data = Vec::new();
    data.extend_from_slice(&discriminators::CPMM_SWAP);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*user_source_token_account, false),
        AccountMeta::new(*user_dest_token_account, false),
        AccountMeta::new(*pool_source_token_account, false),
        AccountMeta::new(*pool_dest_token_account, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Parse pool creation event from transaction
/// 
/// Extracts pool information from a Raydium Initialize/Initialize2 instruction
pub fn parse_pool_creation(
    accounts: &[Pubkey],
    instruction_account_indices: &[u8],
) -> Option<PoolCreationData> {
    // The account order depends on the instruction format
    // This is simplified - verify with actual Raydium IDL
    if instruction_account_indices.len() < 5 {
        return None;
    }

    Some(PoolCreationData {
        pool: accounts.get(instruction_account_indices[0] as usize)?.clone(),
        amm: accounts.get(instruction_account_indices[1] as usize)?.clone(),
        creator: accounts.get(instruction_account_indices[2] as usize)?.clone(),
    })
}

/// Data extracted from a pool creation instruction
#[derive(Debug, Clone)]
pub struct PoolCreationData {
    pub pool: Pubkey,
    pub amm: Pubkey,
    pub creator: Pubkey,
}

/// Check if instruction data matches a pool initialization
pub fn is_pool_initialization(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    let discriminator = &data[0..8];
    discriminator == discriminators::INITIALIZE
        || discriminator == discriminators::INITIALIZE2
        || discriminator == discriminators::CPMM_INITIALIZE
}
