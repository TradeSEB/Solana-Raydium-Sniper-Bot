use anyhow::{Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signature::Signer,
    system_program,
    transaction::VersionedTransaction,
};
use std::str::FromStr;
use tokio::time::{sleep, Duration};

use crate::config::Config;
use crate::detector::{PoolCreationEvent, PoolType};
use crate::instructions::{build_cpmm_swap_instruction, build_raydium_swap_instruction};
use crate::utils;
use crate::wallet::Wallet;

/// Sniper that evaluates and executes buys on new pools
pub struct Sniper {
    rpc_client: RpcClient,
    wallet: Wallet,
    config: Config,
}

impl Sniper {
    pub fn new(rpc_url: String, wallet: Wallet, config: Config) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        );

        Self {
            rpc_client,
            wallet,
            config,
        }
    }

    /// Evaluate if a pool should be sniped based on filters
    pub async fn evaluate_pool(&self, event: &PoolCreationEvent) -> Result<bool> {
        log::info!(
            "Evaluating new pool: pool={}, creator={}, type={:?}",
            event.pool,
            event.creator,
            event.pool_type
        );

        // Check blacklist
        let creator_str = event.creator.to_string();
        if self.config.blacklisted_creators.contains(&creator_str) {
            log::info!("Pool creator is blacklisted: {}", creator_str);
            return Ok(false);
        }

        // Check liquidity
        match self.check_liquidity(&event.pool, &event.pool_type).await {
            Ok(has_sufficient_liquidity) => {
                if !has_sufficient_liquidity {
                    log::info!("Pool does not meet liquidity requirements: {}", event.pool);
                    return Ok(false);
                }
            }
            Err(e) => {
                log::warn!("Failed to check liquidity: {}", e);
                // Continue anyway - liquidity check is optional
            }
        }

        // Check rug indicators
        match self.check_rug_indicators(&event.pool).await {
            Ok(is_safe) => {
                if !is_safe {
                    log::warn!("Pool has rug pull indicators: {}", event.pool);
                    return Ok(false);
                }
            }
            Err(e) => {
                log::warn!("Failed to check rug indicators: {}", e);
                // Continue anyway
            }
        }

        log::info!("Pool passed all filters: {}", event.pool);
        Ok(true)
    }

    /// Check if pool meets liquidity requirements
    async fn check_liquidity(&self, pool: &Pubkey, pool_type: &PoolType) -> Result<bool> {
        // Fetch pool account data
        // Parse to get initial liquidity
        // Compare against min/max thresholds
        
        // Placeholder - implement based on pool account structure
        // You'll need to:
        // 1. Fetch pool account data
        // 2. Deserialize pool account (different for AMM v4 vs CPMM)
        // 3. Extract token reserves
        // 4. Calculate USD value
        // 5. Check against config thresholds
        
        // For now, return true (passes check)
        Ok(true)
    }

    /// Check rug pull indicators
    async fn check_rug_indicators(&self, pool: &Pubkey) -> Result<bool> {
        // Fetch pool account to get token mints
        // Then check each mint for:
        // - Mint authority (should be None)
        // - Freeze authority (should be None)
        // - LP token supply/burn status
        
        // Placeholder - implement actual checks
        Ok(true)
    }

    /// Execute a buy on a pool
    pub async fn execute_buy(&self, event: &PoolCreationEvent) -> Result<String> {
        if self.config.dry_run {
            log::info!(
                "[DRY RUN] Would buy from pool: pool={}, amount={} SOL",
                event.pool,
                self.config.buy_amount_sol
            );
            return Ok("dry_run_simulation".to_string());
        }

        log::info!(
            "Executing buy: pool={}, amount={} SOL, type={:?}",
            event.pool,
            self.config.buy_amount_sol,
            event.pool_type
        );

        // Get latest blockhash
        let (blockhash, _) = self
            .rpc_client
            .get_latest_blockhash()
            .await
            .context("Failed to get latest blockhash")?;

        // Build swap instruction based on pool type
        let buy_amount_lamports = utils::sol_to_lamports(self.config.buy_amount_sol);
        
        // Calculate min amount out with slippage
        // Note: This is simplified - you should calculate based on pool reserves
        let estimated_amount_out = buy_amount_lamports; // Placeholder
        let min_amount_out = utils::calculate_min_amount_out(
            estimated_amount_out,
            self.config.slippage_bps,
        );

        let swap_ix = match event.pool_type {
            PoolType::AMMv4 => {
                // Build AMM v4 swap instruction
                // Note: This requires deriving many accounts - simplified here
                self.build_amm_v4_swap(&event.pool, buy_amount_lamports, min_amount_out)
                    .await?
            }
            PoolType::CPMM => {
                // Build CPMM swap instruction
                self.build_cpmm_swap(&event.pool, buy_amount_lamports, min_amount_out)
                    .await?
            }
        };

        // Build transaction
        let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
            &[swap_ix],
            Some(&self.wallet.pubkey()),
        );

        // Add priority fee instruction
        let priority_fee = utils::estimate_priority_fee(
            &self.rpc_client,
            self.config.priority_fee_micro_lamports,
        )
        .await;

        let priority_fee_ix = ComputeBudgetInstruction::set_compute_unit_price(priority_fee);
        let compute_unit_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.max_compute_units,
        );

        transaction.instructions.insert(0, priority_fee_ix);
        transaction.instructions.insert(1, compute_unit_limit_ix);

        transaction.sign(&[self.wallet.keypair()], blockhash);

        // Convert to VersionedTransaction
        let versioned_tx = VersionedTransaction::from(transaction);

        // Send with retry
        self.send_transaction_with_retry(versioned_tx, 3).await
    }

    /// Build AMM v4 swap instruction
    async fn build_amm_v4_swap(
        &self,
        _pool: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<solana_sdk::instruction::Instruction> {
        // This is a placeholder - actual implementation requires:
        // 1. Fetching pool account to get all required accounts
        // 2. Deriving associated token accounts
        // 3. Getting AMM, authority, open orders, etc.
        // 4. Building the full instruction with all accounts
        
        // For now, return a simplified version
        anyhow::bail!("AMM v4 swap instruction building not fully implemented - requires pool account parsing");
    }

    /// Build CPMM swap instruction
    async fn build_cpmm_swap(
        &self,
        pool: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<solana_sdk::instruction::Instruction> {
        // Fetch pool account to get token accounts
        // This is simplified - actual implementation requires parsing pool account
        
        // Placeholder accounts
        let user_source = self.wallet.pubkey();
        let user_dest = self.wallet.pubkey();
        let pool_source = *pool; // Placeholder
        let pool_dest = *pool; // Placeholder

        build_cpmm_swap_instruction(
            &self.wallet.pubkey(),
            pool,
            &user_source,
            &user_dest,
            &pool_source,
            &pool_dest,
            amount_in,
            min_amount_out,
        )
    }

    /// Send transaction with retry logic
    async fn send_transaction_with_retry(
        &self,
        transaction: VersionedTransaction,
        max_retries: u32,
    ) -> Result<String> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            log::info!("Sending buy transaction (attempt {}/{})", attempt, max_retries);

            match self.rpc_client.send_transaction(&transaction).await {
                Ok(signature) => {
                    log::info!("Buy transaction sent: {}", signature);
                    
                    // Wait for confirmation
                    if let Err(e) = self.wait_for_confirmation(&signature).await {
                        log::warn!("Transaction sent but confirmation error: {}", e);
                    }

                    return Ok(signature.to_string());
                }
                Err(e) => {
                    log::warn!("Transaction send failed (attempt {}): {}", attempt, e);
                    last_error = Some(e);

                    if attempt < max_retries {
                        let delay = Duration::from_millis(1000 * attempt as u64);
                        log::info!("Retrying in {:?}...", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("Transaction failed after {} retries", max_retries))
            .into())
    }

    /// Wait for transaction confirmation
    async fn wait_for_confirmation(
        &self,
        signature: &solana_sdk::signature::Signature,
    ) -> Result<()> {
        const MAX_WAIT_TIME: Duration = Duration::from_secs(30);
        const POLL_INTERVAL: Duration = Duration::from_millis(500);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > MAX_WAIT_TIME {
                anyhow::bail!("Transaction confirmation timeout");
            }

            match self.rpc_client.get_signature_status(signature).await {
                Ok(Some(status)) => {
                    if status.err.is_some() {
                        anyhow::bail!("Transaction failed: {:?}", status.err);
                    }
                    if status.confirmation_status.is_some() {
                        log::info!("Transaction confirmed: {}", signature);
                        return Ok(());
                    }
                }
                Ok(None) => {
                    // Still processing
                }
                Err(e) => {
                    log::warn!("Error checking transaction status: {}", e);
                }
            }

            sleep(POLL_INTERVAL).await;
        }
    }

    /// Get wallet balance
    pub async fn get_balance(&self) -> Result<u64> {
        let balance = self
            .rpc_client
            .get_balance(&self.wallet.pubkey())
            .await
            .context("Failed to get account balance")?;

        Ok(balance)
    }
}
