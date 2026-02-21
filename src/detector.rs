use anyhow::{Context, Result};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio_stream::StreamExt;

use crate::config::{Config, RAYDIUM_AMM_V4_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID};
use crate::instructions::{is_pool_initialization, PoolCreationData};

/// New pool creation event detected from Raydium
#[derive(Debug, Clone)]
pub struct PoolCreationEvent {
    pub pool: Pubkey,
    pub amm: Pubkey,
    pub creator: Pubkey,
    pub program_id: Pubkey,
    pub signature: String,
    pub slot: u64,
    pub timestamp: i64,
    pub pool_type: PoolType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolType {
    AMMv4,
    CPMM,
}

/// Pool detector using Yellowstone Geyser gRPC or WebSocket fallback
pub struct PoolDetector {
    config: Config,
    amm_v4_program_id: Pubkey,
    cpmm_program_id: Pubkey,
}

impl PoolDetector {
    pub fn new(config: Config) -> Result<Self> {
        let amm_v4_program_id = Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM_ID)
            .context("Failed to parse Raydium AMM v4 program ID")?;
        let cpmm_program_id = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)
            .context("Failed to parse Raydium CPMM program ID")?;

        Ok(Self {
            config,
            amm_v4_program_id,
            cpmm_program_id,
        })
    }

    /// Start detecting new pool creations
    /// 
    /// Returns a stream of PoolCreationEvent
    pub async fn start_detection(
        &self,
    ) -> Result<tokio_stream::wrappers::ReceiverStream<PoolCreationEvent>> {
        // Try Yellowstone Geyser gRPC first if configured
        if let Some(ref grpc_url) = self.config.yellowstone_grpc_url {
            log::info!("Attempting to connect to Yellowstone Geyser gRPC: {}", grpc_url);
            match self.start_geyser_stream(grpc_url).await {
                Ok(stream) => {
                    log::info!("Successfully connected to Yellowstone Geyser");
                    return Ok(stream);
                }
                Err(e) => {
                    log::warn!("Failed to connect to Yellowstone Geyser: {}", e);
                    if !self.config.use_websocket_fallback {
                        return Err(e);
                    }
                    log::info!("Falling back to WebSocket subscription");
                }
            }
        }

        // Fallback to WebSocket subscription
        log::info!("Using WebSocket subscription as detection method");
        self.start_websocket_subscription().await
    }

    /// Start Yellowstone Geyser gRPC stream
    async fn start_geyser_stream(
        &self,
        grpc_url: &str,
    ) -> Result<tokio_stream::wrappers::ReceiverStream<PoolCreationEvent>> {
        use yellowstone_grpc::{
            geyser::SubscribeRequest,
            proto::geyser::SubscribeRequestFilterAccounts,
        };

        let (tx, rx) = tokio::sync::mpsc::channel(1000);

        // Create gRPC client
        let mut client = yellowstone_grpc::GeyserGrpcClient::connect(grpc_url)
            .await
            .context("Failed to connect to Yellowstone Geyser")?;

        // Build program IDs to monitor
        let mut program_ids = Vec::new();
        if self.config.monitor_amm_v4 {
            program_ids.push(self.amm_v4_program_id.to_string());
        }
        if self.config.monitor_cpmm {
            program_ids.push(self.cpmm_program_id.to_string());
        }

        if program_ids.is_empty() {
            anyhow::bail!("No Raydium programs enabled for monitoring");
        }

        // Subscribe to Raydium program transactions
        let filter = SubscribeRequestFilterAccounts {
            account: vec![],
            owner: program_ids,
            filters: vec![],
        };

        let request = SubscribeRequest {
            slots: vec![],
            accounts: vec![filter],
            transactions: vec![],
            transactions_status: vec![],
            blocks: vec![],
            blocks_meta: vec![],
            accounts_data_slice: vec![],
            commitment: Some(yellowstone_grpc::proto::geyser::CommitmentLevel::Confirmed as i32),
        };

        // Spawn task to handle stream
        let mut stream = client
            .subscribe_once(request)
            .await
            .context("Failed to subscribe to Geyser stream")?;

        let amm_v4_program_id = self.amm_v4_program_id;
        let cpmm_program_id = self.cpmm_program_id;
        let config = self.config.clone();

        tokio::spawn(async move {
            while let Some(msg) = stream.message().await.transpose() {
                match msg {
                    Ok(update) => {
                        // Parse transaction update
                        if let Some(tx_update) = update.transaction {
                            if let Some(event) = Self::parse_transaction_update(
                                &tx_update,
                                &amm_v4_program_id,
                                &cpmm_program_id,
                            ) {
                                if let Err(e) = tx.send(event).await {
                                    log::error!("Failed to send pool creation event: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Error receiving Geyser update: {}", e);
                    }
                }
            }
        });

        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    /// Start WebSocket subscription (fallback method)
    async fn start_websocket_subscription(
        &self,
    ) -> Result<tokio_stream::wrappers::ReceiverStream<PoolCreationEvent>> {
        use solana_client::nonblocking::rpc_client::RpcClient;

        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        let rpc_url = self.config.rpc_url.clone();
        let amm_v4_program_id = self.amm_v4_program_id;
        let cpmm_program_id = self.cpmm_program_id;
        let config = self.config.clone();

        // Spawn task to poll for new transactions
        tokio::spawn(async move {
            let client = RpcClient::new(rpc_url);
            let mut last_signatures: std::collections::HashSet<String> = std::collections::HashSet::new();

            loop {
                // Get recent signatures for both programs
                let mut program_ids = Vec::new();
                if config.monitor_amm_v4 {
                    program_ids.push(amm_v4_program_id);
                }
                if config.monitor_cpmm {
                    program_ids.push(cpmm_program_id);
                }

                for program_id in &program_ids {
                    match client.get_signatures_for_address(program_id).await {
                        Ok(signatures) => {
                            for sig_info in signatures.iter().take(10) {
                                // Skip if we've already processed this
                                if last_signatures.contains(&sig_info.signature) {
                                    continue;
                                }

                                // Parse transaction
                                if let Ok(tx_data) = client
                                    .get_transaction(
                                        &sig_info.signature,
                                        solana_transaction_status::UiTransactionEncoding::Json,
                                    )
                                    .await
                                {
                                    if let Some(event) = Self::parse_transaction(
                                        &tx_data,
                                        program_id,
                                        &sig_info.signature,
                                        &amm_v4_program_id,
                                        &cpmm_program_id,
                                    ) {
                                        if let Err(e) = tx.send(event).await {
                                            log::error!("Failed to send pool creation event: {}", e);
                                        }
                                    }
                                }

                                last_signatures.insert(sig_info.signature.clone());
                            }
                        }
                        Err(e) => {
                            log::warn!("Error fetching signatures: {}", e);
                        }
                    }
                }

                // Rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(config.rate_limit_ms)).await;
            }
        });

        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    /// Parse transaction update from Geyser
    fn parse_transaction_update(
        _update: &yellowstone_grpc::proto::geyser::TransactionUpdate,
        _amm_v4_program_id: &Pubkey,
        _cpmm_program_id: &Pubkey,
    ) -> Option<PoolCreationEvent> {
        // Parse Geyser transaction update
        // This is a simplified version - actual implementation depends on Geyser message format
        // You'll need to:
        // 1. Extract transaction data
        // 2. Check for pool initialization instruction
        // 3. Extract accounts (pool, amm, creator)
        // 4. Return PoolCreationEvent
        
        // Placeholder - implement based on actual Geyser message structure
        None
    }

    /// Parse transaction from RPC
    fn parse_transaction(
        tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        program_id: &Pubkey,
        signature: &str,
        amm_v4_program_id: &Pubkey,
        cpmm_program_id: &Pubkey,
    ) -> Option<PoolCreationEvent> {
        use solana_transaction_status::UiTransactionEncoding;

        // Extract transaction data
        let transaction = match tx.transaction {
            solana_transaction_status::EncodedTransaction::Json(ref json_tx) => json_tx,
            _ => return None,
        };

        // Find instructions for Raydium program
        if let Some(ref message) = transaction.message {
            if let Some(ref account_keys) = message.account_keys {
                // Find program index
                let program_index = account_keys
                    .iter()
                    .position(|key| key == program_id.to_string())?;

                // Find pool initialization instruction
                if let Some(ref instructions) = message.instructions {
                    for ix in instructions {
                        if let Some(program_id_index) = ix.program_id_index {
                            if program_id_index as usize == program_index {
                                // Check if this is a pool initialization
                                if let Some(pool_data) = Self::parse_pool_instruction(
                                    &ix.data,
                                    account_keys,
                                    &ix.accounts,
                                    program_id,
                                    amm_v4_program_id,
                                    cpmm_program_id,
                                ) {
                                    return Some(PoolCreationEvent {
                                        pool: pool_data.pool,
                                        amm: pool_data.amm,
                                        creator: pool_data.creator,
                                        program_id: *program_id,
                                        signature: signature.to_string(),
                                        slot: tx.slot.unwrap_or(0),
                                        timestamp: chrono::Utc::now().timestamp(),
                                        pool_type: if program_id == amm_v4_program_id {
                                            PoolType::AMMv4
                                        } else {
                                            PoolType::CPMM
                                        },
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Parse pool initialization instruction
    fn parse_pool_instruction(
        data: &str,
        account_keys: &[String],
        account_indices: &[u8],
        program_id: &Pubkey,
        _amm_v4_program_id: &Pubkey,
        _cpmm_program_id: &Pubkey,
    ) -> Option<PoolCreationData> {
        // Decode base58 instruction data
        let decoded = bs58::decode(data).into_vec().ok()?;

        // Check if it's a pool initialization
        if !is_pool_initialization(&decoded) {
            return None;
        }

        // Extract accounts (order depends on instruction format)
        // This is simplified - verify with actual Raydium IDL
        if account_indices.len() < 4 {
            return None;
        }

        let pool = Pubkey::from_str(account_keys.get(account_indices[0] as usize)?).ok()?;
        let amm = Pubkey::from_str(account_keys.get(account_indices[1] as usize)?).ok()?;
        let creator = Pubkey::from_str(account_keys.get(account_indices[2] as usize)?).ok()?;

        Some(PoolCreationData {
            pool,
            amm,
            creator,
        })
    }
}
