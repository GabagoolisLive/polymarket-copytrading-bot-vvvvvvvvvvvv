use anyhow::Result;
use alloy::signers::local::PrivateKeySigner;
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::EnvConfig;
use crate::types::{RtdsActivity, UserActivity, UserPosition};
use crate::utils::{fetch_data, get_usdc_balance, post_order, Logger};

type ProcessedTrades = Arc<Mutex<HashSet<String>>>;

static RUNNING: AtomicBool = AtomicBool::new(true);

pub fn stop_trade_executor() {
    RUNNING.store(false, Ordering::SeqCst);
    Logger::info("Trade executor shutdown requested...");
}

pub async fn execute_trade(
    config: Arc<EnvConfig>,
    activity: RtdsActivity,
    address: String,
    http_client: Arc<reqwest::Client>,
    clob_client: Arc<ClobClient<Authenticated<Normal>>>,
    signer: Arc<Mutex<PrivateKeySigner>>,
    processed_trades: ProcessedTrades,
) -> Result<()> {
    let ts = activity.timestamp.unwrap_or(0);
    let ts_ms = if ts > 1_000_000_000_000 {
        ts
    } else {
        ts * 1000
    };
    let hours_ago = (chrono::Utc::now().timestamp_millis() - ts_ms) as f64 / (1000.0 * 3600.0);
    if hours_ago > config.too_old_timestamp_hours as f64 {
        return Ok(());
    }

    let tx_hash = activity.transaction_hash.as_deref().unwrap_or("");
    if tx_hash.is_empty() {
        return Ok(());
    }
    
    let trade_key = format!("{}:{}", address, tx_hash);
    {
        let mut processed = processed_trades.lock().await;
        if processed.contains(&trade_key) {
            return Ok(());
        }
        processed.insert(trade_key.clone());
        if processed.len() > 1000 {
            processed.clear();
            processed.insert(trade_key);
        }
    }

    Logger::info(&format!(
        "New trade detected for {} - executing immediately",
        Logger::format_address(&address)
    ));

    let trade = UserActivity {
        id: None,
        proxy_wallet: activity.proxy_wallet.clone(),
        timestamp: activity.timestamp,
        condition_id: activity.condition_id.clone(),
        activity_type: Some("TRADE".to_string()),
        size: activity.size,
        usdc_size: Some(activity.usdc_size()),
        transaction_hash: activity.transaction_hash.clone(),
        price: activity.price,
        asset: activity.asset.clone(),
        side: activity.side.clone(),
        outcome_index: activity.outcome_index,
        title: activity.title.clone(),
        slug: activity.slug.clone(),
        icon: activity.icon.clone(),
        event_slug: activity.event_slug.clone(),
        outcome: activity.outcome.clone(),
        name: activity.name.clone(),
        pseudonym: None,
        bio: None,
        profile_image: None,
        profile_image_optimized: None,
        bot: Some(false),
        bot_executed_time: Some(0),
        my_bought_size: None,
    };

    Logger::trade(
        &address,
        trade.side.as_deref().unwrap_or("UNKNOWN"),
        crate::utils::TradeDetails {
            asset: trade.asset.clone(),
            side: trade.side.clone(),
            amount: trade.usdc_size,
            price: trade.price,
            slug: trade.slug.clone(),
            event_slug: trade.event_slug.clone(),
            transaction_hash: trade.transaction_hash.clone(),
            title: trade.title.clone(),
        },
    );

    let my_positions_url = format!(
        "https://data-api.polymarket.com/positions?user={}",
        config.proxy_wallet
    );
    let user_positions_url = format!(
        "https://data-api.polymarket.com/positions?user={}",
        address
    );

    let my_positions_data: serde_json::Value = fetch_data(
        &http_client,
        &my_positions_url,
        config.request_timeout_ms,
        config.network_retry_limit,
    )
    .await?;
    let user_positions_data: serde_json::Value = fetch_data(
        &http_client,
        &user_positions_url,
        config.request_timeout_ms,
        config.network_retry_limit,
    )
    .await?;

    let my_positions: Vec<UserPosition> = if let Some(arr) = my_positions_data.as_array() {
        arr.iter()
            .filter_map(|p| serde_json::from_value::<UserPosition>(p.clone()).ok())
            .collect()
    } else {
        Vec::new()
    };

    let user_positions: Vec<UserPosition> = if let Some(arr) = user_positions_data.as_array() {
        arr.iter()
            .filter_map(|p| serde_json::from_value::<UserPosition>(p.clone()).ok())
            .collect()
    } else {
        Vec::new()
    };

    let condition_id = trade.condition_id.as_deref();
    let my_position = my_positions
        .iter()
        .find(|p| p.condition_id.as_deref() == condition_id);
    let user_position = user_positions
        .iter()
        .find(|p| p.condition_id.as_deref() == condition_id);

    let my_balance = get_usdc_balance(
        &config.rpc_url,
        &config.usdc_contract_address,
        &config.proxy_wallet,
    )
    .await
    .unwrap_or(0.0);

    let user_balance: f64 = user_positions
        .iter()
        .map(|p| p.current_value.unwrap_or(0.0))
        .sum();

    Logger::balance(my_balance, user_balance, &address);

    let condition = if trade.side.as_deref().unwrap_or("") == "BUY" {
        "buy"
    } else {
        "sell"
    };

    let mut signer_guard = signer.lock().await;
    post_order(
        &config,
        &clob_client,
        condition,
        my_position,
        user_position,
        &trade,
        my_balance,
        user_balance,
        &address,
        &http_client,
        &mut *signer_guard,
    )
    .await?;

    Logger::separator();
    Ok(())
}

pub async fn run_trade_executor(
    config: Arc<EnvConfig>,
    http_client: Arc<reqwest::Client>,
    clob_client: Arc<ClobClient<Authenticated<Normal>>>,
    signer: Arc<Mutex<PrivateKeySigner>>,
    mut rx: tokio::sync::mpsc::Receiver<(RtdsActivity, String)>,
) -> Result<()> {
    RUNNING.store(true, Ordering::SeqCst);
    let processed_trades: ProcessedTrades = Arc::new(Mutex::new(HashSet::new()));

    Logger::success("Trade executor started - ready to execute trades");

    while RUNNING.load(Ordering::SeqCst) {
        match rx.recv().await {
            Some((activity, address)) => {
                if let Err(e) = execute_trade(
                    config.clone(),
                    activity,
                    address,
                    http_client.clone(),
                    clob_client.clone(),
                    signer.clone(),
                    processed_trades.clone(),
                )
                .await
                {
                    Logger::error(&format!("Error executing trade: {}", e));
                }
            }
            None => {
                Logger::warning("Trade channel closed");
                break;
            }
        }
    }

    Ok(())
}

