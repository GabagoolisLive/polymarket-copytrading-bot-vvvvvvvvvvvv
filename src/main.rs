mod config;
mod executor;
mod monitor;
mod types;
mod utils;

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;

use config::EnvConfig;
use executor::{run_trade_executor, stop_trade_executor};
use monitor::{run_trade_monitor, stop_trade_monitor};
use utils::{create_clob_client, get_usdc_balance, is_contract_address, perform_health_check, Logger};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    println!();
    println!(
        "  {} New here? Read GETTING_STARTED.md and run a health check.{}",
        utils::theme::colors::MUTED,
        utils::theme::colors::RESET
    );
    println!();

    let config = EnvConfig::from_env().await?;

    Logger::startup(&config.user_addresses, &config.proxy_wallet);

    Logger::info("Running system check…");
    let balance = get_usdc_balance(
        &config.rpc_url,
        &config.usdc_contract_address,
        &config.proxy_wallet,
    )
    .await;
    let polymarket_ok = utils::fetch_data(
        &reqwest::Client::new(),
        "https://data-api.polymarket.com/positions?user=0x0000000000000000000000000000000000000000",
        config.request_timeout_ms,
        config.network_retry_limit,
    )
    .await
    .is_ok();
    let health = perform_health_check(&config.rpc_url, balance, polymarket_ok).await;

    Logger::separator();
    Logger::header("SYSTEM CHECK");
    let overall = if health.healthy {
        "All systems go"
    } else {
        "Degraded — check items below"
    };
    Logger::health_line(
        "Overall",
        if health.healthy { "ok" } else { "error" },
        overall,
    );
    Logger::health_line("RPC", &health.checks.rpc.status, &health.checks.rpc.message);
    Logger::health_line(
        "Balance",
        &health.checks.balance.status,
        &health.checks.balance.message,
    );
    Logger::health_line(
        "Polymarket API",
        &health.checks.polymarket_api.status,
        &health.checks.polymarket_api.message,
    );
    Logger::separator();

    if !health.healthy {
        Logger::warning("System check reported issues; continuing anyway.");
    }

    Logger::info("Initializing CLOB client...");
    let is_proxy_safe = is_contract_address(&config.rpc_url, &config.proxy_wallet)
        .await
        .unwrap_or(false);
    let wallet_type = if is_proxy_safe {
        "Gnosis Safe"
    } else {
        "EOA (Externally Owned Account)"
    };
    Logger::info(&format!("Wallet type detected: {}", wallet_type));
    Logger::success("CLOB client ready");

    Logger::separator();
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(config.request_timeout_ms))
        .build()?;

    Logger::info("Initializing executor...");
    let (clob_client, signer) = create_clob_client(&config).await?;
    let clob_client = Arc::new(clob_client);
    let signer = Arc::new(tokio::sync::Mutex::new(signer));
    let config_arc = Arc::new(config.clone());
    let http_arc = Arc::new(http_client.clone());

    let (tx, rx) = tokio::sync::mpsc::channel::<(crate::types::RtdsActivity, String)>(100);

    let config_exec = config_arc.clone();
    let http_exec = http_arc.clone();
    let clob_exec = clob_client.clone();
    let signer_exec = signer.clone();
    let rx_exec = rx;
    tokio::spawn(async move {
        if let Err(e) = run_trade_executor(config_exec, http_exec, clob_exec, signer_exec, rx_exec).await {
            Logger::error(&format!("Executor error: {}", e));
        }
    });

    Logger::info("Starting trade monitor...");
    let _monitor_handle = run_trade_monitor(&config, &http_client, tx).await?;

    match signal::ctrl_c().await {
        Ok(()) => {
            Logger::separator();
            Logger::info("Shutdown requested. Stopping…");
        }
        Err(_) => {}
    }

    stop_trade_monitor();
    stop_trade_executor();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Logger::success("Goodbye.");
    Ok(())
}
