use anyhow::Result;
use log::LevelFilter;

/// Initialize logging based on log level string
pub fn init_logging(log_level: &str) -> Result<()> {
    let filter = match log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    env_logger::Builder::from_default_env()
        .filter_level(filter)
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    Ok(())
}

/// Format lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Format SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

/// Rate limiter helper - simple delay
pub async fn rate_limit_delay(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}

/// Estimate priority fee dynamically
/// 
/// In production, you should query get_recent_prioritization_fees
pub async fn estimate_priority_fee(
    _rpc_client: &solana_client::nonblocking::rpc_client::RpcClient,
    base_fee_micro_lamports: u64,
) -> u64 {
    // Simple heuristic - in production, query recent fees
    // For now, return base fee with some jitter
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let jitter = rng.gen_range(0..50_000);
    base_fee_micro_lamports + jitter
}

/// Calculate minimum amount out with slippage
pub fn calculate_min_amount_out(amount_out: u64, slippage_bps: u16) -> u64 {
    let slippage_factor = (10000 - slippage_bps) as f64 / 10000.0;
    (amount_out as f64 * slippage_factor) as u64
}

/// Estimate USD value from SOL amount (simplified)
/// 
/// In production, fetch current SOL price from an oracle
pub fn estimate_usd_value_sol(sol_amount: f64) -> f64 {
    // Placeholder - use current SOL price (~$100-200 as of 2026)
    // In production, fetch from price oracle
    sol_amount * 150.0 // Approximate
}

/// Check if mint has rug pull indicators
/// 
/// Basic checks: mint authority, freeze authority
pub async fn check_rug_indicators(
    _rpc_client: &solana_client::nonblocking::rpc_client::RpcClient,
    _mint: &solana_sdk::pubkey::Pubkey,
) -> Result<bool> {
    // Placeholder - implement actual checks:
    // 1. Fetch mint account
    // 2. Check if mint_authority is None (good - cannot mint more)
    // 3. Check if freeze_authority is None (good - cannot freeze)
    // 4. Return true if safe, false if risky
    
    // For now, return true (safe)
    Ok(true)
}
