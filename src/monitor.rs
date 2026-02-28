use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::EnvConfig;
use crate::types::{RtdsActivity, UserPosition};
use crate::utils::{fetch_data, get_usdc_balance, Logger};

const RTDS_URL: &str = "wss://ws-live-data.polymarket.com";
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
const RECONNECT_DELAY_SECS: u64 = 5;

static RUNNING: AtomicBool = AtomicBool::new(true);

pub fn stop_trade_monitor() {
    RUNNING.store(false, Ordering::SeqCst);
    Logger::info("Trade monitor shutdown requested...");
}

pub struct TradeMonitorHandle {
    _tx: broadcast::Sender<()>,
}

async fn init(
    config: &EnvConfig,
    http_client: &reqwest::Client,
) -> Result<()> {
    let my_positions_url = format!(
        "https://data-api.polymarket.com/positions?user={}",
        config.proxy_wallet
    );
    let current_balance = get_usdc_balance(
        &config.rpc_url,
        &config.usdc_contract_address,
        &config.proxy_wallet,
    )
    .await
    .unwrap_or(0.0);

    match fetch_data(
        http_client,
        &my_positions_url,
        config.request_timeout_ms,
        config.network_retry_limit,
    )
    .await
    {
        Ok(data) => {
            if let Some(arr) = data.as_array() {
                let mut total_value = 0.0;
                let mut initial_value = 0.0;
                let mut weighted_pnl = 0.0;
                for pos in arr.iter() {
                    let value = pos
                        .get("currentValue")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let initial = pos
                        .get("initialValue")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let pnl = pos
                        .get("percentPnl")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    total_value += value;
                    initial_value += initial;
                    weighted_pnl += value * pnl;
                }
                let my_overall_pnl = if total_value > 0.0 {
                    weighted_pnl / total_value
                } else {
                    0.0
                };

                let mut top_positions = arr.clone();
                top_positions.sort_by(|a, b| {
                    let pnl_a = a
                        .get("percentPnl")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let pnl_b = b
                        .get("percentPnl")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    pnl_b.partial_cmp(&pnl_a).unwrap_or(std::cmp::Ordering::Equal)
                });
                let top_positions: Vec<_> = top_positions.iter().take(5).cloned().collect();

                Logger::clear_line();
                Logger::my_positions(
                    &config.proxy_wallet,
                    arr.len(),
                    &top_positions,
                    my_overall_pnl,
                    total_value,
                    initial_value,
                    current_balance,
                );
            } else {
                Logger::clear_line();
                Logger::my_positions(
                    &config.proxy_wallet,
                    0,
                    &[],
                    0.0,
                    0.0,
                    0.0,
                    current_balance,
                );
            }
        }
        Err(e) => {
            Logger::error(&format!("Failed to fetch your positions: {}", e));
        }
    }

    let mut position_counts = Vec::new();
    let mut position_details = Vec::new();
    let mut profitabilities = Vec::new();
    for addr in &config.user_addresses {
        match fetch_data(
            http_client,
            &format!("https://data-api.polymarket.com/positions?user={}", addr),
            config.request_timeout_ms,
            config.network_retry_limit,
        )
        .await
        {
            Ok(data) => {
                if let Some(arr) = data.as_array() {
                    let positions: Vec<UserPosition> = arr
                        .iter()
                        .filter_map(|p| serde_json::from_value::<UserPosition>(p.clone()).ok())
                        .collect();
                    position_counts.push(positions.len());

                    let mut total_value = 0.0;
                    let mut weighted_pnl = 0.0;
                    for pos in &positions {
                        let value = pos.current_value.unwrap_or(0.0);
                        let pnl = pos.percent_pnl.unwrap_or(0.0);
                        total_value += value;
                        weighted_pnl += value * pnl;
                    }
                    let overall_pnl = if total_value > 0.0 {
                        weighted_pnl / total_value
                    } else {
                        0.0
                    };
                    profitabilities.push(overall_pnl);

                    let mut sorted_positions = positions.clone();
                    sorted_positions.sort_by(|a, b| {
                        let pnl_a = a.percent_pnl.unwrap_or(0.0);
                        let pnl_b = b.percent_pnl.unwrap_or(0.0);
                        pnl_b.partial_cmp(&pnl_a).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let top_positions: Vec<serde_json::Value> = sorted_positions
                        .iter()
                        .take(3)
                        .filter_map(|p| serde_json::to_value(p).ok())
                        .collect();
                    position_details.push(top_positions);
                } else {
                    position_counts.push(0);
                    position_details.push(Vec::new());
                    profitabilities.push(0.0);
                }
            }
            Err(_) => {
                position_counts.push(0);
                position_details.push(Vec::new());
                profitabilities.push(0.0);
            }
        }
    }
    Logger::clear_line();
    Logger::traders_positions(
        &config.user_addresses,
        &position_counts,
        &position_details,
        &profitabilities,
    );

    Ok(())
}


async fn connect_rtds(
    config: Arc<EnvConfig>,
    tx: tokio::sync::mpsc::Sender<(RtdsActivity, String)>,
    reconnect_attempts: Arc<std::sync::atomic::AtomicU32>,
) -> Result<()> {
    loop {
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }

        Logger::info(&format!("Connecting to RTDS at {}...", RTDS_URL));

        match connect_async(RTDS_URL).await {
            Ok((ws_stream, _)) => {
                Logger::success("RTDS WebSocket connected");
                reconnect_attempts.store(0, Ordering::SeqCst);

                let (mut write, mut read) = ws_stream.split();

                let subscriptions: Vec<_> = config
                    .user_addresses
                    .iter()
                    .map(|_| json!({"topic": "activity", "type": "trades"}))
                    .collect();

                let subscribe_message = json!({
                    "action": "subscribe",
                    "subscriptions": subscriptions
                });

                if let Err(e) = write.send(Message::Text(subscribe_message.to_string())).await {
                    Logger::error(&format!("Failed to send subscription: {}", e));
                    continue;
                }

                Logger::success(&format!(
                    "Subscribed to RTDS for {} trader(s) - monitoring trades in real-time",
                    config.user_addresses.len()
                ));

                let config_msg = config.clone();
                let tx_msg = tx.clone();
                let message_task = tokio::spawn(async move {
                    while RUNNING.load(Ordering::SeqCst) {
                        match read.next().await {
                            Some(Ok(Message::Text(t))) => {
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&t) {
                                    if parsed.get("action").and_then(|a| a.as_str()) == Some("subscribed")
                                        || parsed.get("status").and_then(|s| s.as_str()) == Some("subscribed")
                                    {
                                        Logger::info("RTDS subscription confirmed");
                                        continue;
                                    }

                                    if parsed.get("topic").and_then(|t| t.as_str()) == Some("activity")
                                        && parsed.get("type").and_then(|t| t.as_str()) == Some("trades")
                                    {
                                        if let Some(payload) = parsed.get("payload") {
                                            if let Ok(activity) =
                                                serde_json::from_value::<RtdsActivity>(payload.clone())
                                            {
                                                let proxy = activity
                                                    .proxy_wallet
                                                    .as_deref()
                                                    .unwrap_or("")
                                                    .to_lowercase();
                                                if config_msg
                                                    .user_addresses
                                                    .iter()
                                                    .any(|a| a.to_lowercase() == proxy)
                                                {
                                                    if let Err(e) = tx_msg.send((activity, proxy)).await {
                                                        Logger::error(&format!(
                                                            "Error sending trade to executor: {}",
                                                            e
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) => {
                                Logger::warning("RTDS WebSocket closed");
                                break;
                            }
                            Some(Err(e)) => {
                                Logger::error(&format!("RTDS WebSocket error: {}", e));
                                break;
                            }
                            None => break,
                            _ => continue,
                        }
                    }
                });

                message_task.await.ok();
            }
            Err(e) => {
                Logger::error(&format!("Failed to connect to RTDS: {}", e));
            }
        }

        if RUNNING.load(Ordering::SeqCst) {
            let attempts = reconnect_attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if attempts < MAX_RECONNECT_ATTEMPTS {
                let delay = RECONNECT_DELAY_SECS * attempts.min(5) as u64;
                Logger::info(&format!(
                    "Reconnecting to RTDS in {}s (attempt {}/{})...",
                    delay, attempts, MAX_RECONNECT_ATTEMPTS
                ));
                sleep(Duration::from_secs(delay)).await;
            } else {
                Logger::error(&format!(
                    "Max reconnection attempts ({}) reached. Please restart the bot.",
                    MAX_RECONNECT_ATTEMPTS
                ));
                break;
            }
        }
    }

    Ok(())
}

pub async fn run_trade_monitor(
    config: &EnvConfig,
    http_client: &reqwest::Client,
    tx: tokio::sync::mpsc::Sender<(RtdsActivity, String)>,
) -> Result<TradeMonitorHandle> {
    RUNNING.store(true, Ordering::SeqCst);

    init(config, http_client).await?;

    Logger::success(&format!(
        "Monitoring {} trader(s) using RTDS (Real-Time Data Stream)",
        config.user_addresses.len()
    ));
    Logger::info("Trades will be sent to executor for immediate execution.");
    Logger::separator();

    let config_arc = Arc::new(config.clone());
    let reconnect_attempts = Arc::new(std::sync::atomic::AtomicU32::new(0));

    let config_ws = config_arc.clone();
    let tx_ws = tx.clone();
    let reconnect_ws = reconnect_attempts.clone();
    tokio::spawn(async move {
        let _ = connect_rtds(config_ws, tx_ws, reconnect_ws).await;
    });

    let (broadcast_tx, _) = broadcast::channel::<()>(1);
    Ok(TradeMonitorHandle { _tx: broadcast_tx })
}
