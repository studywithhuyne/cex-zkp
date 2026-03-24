use std::{collections::HashMap, str::FromStr, time::{Duration, Instant}};

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use rand::Rng;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::state::{AppState, SimulatorPairStats, SimulatorProfile};

const PAIRS: [(&str, &str, &str); 4] = [
    ("BTC_USDT", "BTC", "USDT"),
    ("ETH_USDT", "ETH", "USDT"),
    ("SOL_USDT", "SOL", "USDT"),
    ("BNB_USDT", "BNB", "USDT"),
];

const ORDER_ENDPOINT: &str = "http://127.0.0.1:3000/api/orders";
const LIVE_TICKERS_ENDPOINT: &str = "http://127.0.0.1:3000/api/market/tickers/live?symbols=BTCUSDT,ETHUSDT,SOLUSDT,BNBUSDT";
const ANCHOR_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy)]
struct ProfileConfig {
    interval_ms: u64,
    orders_per_pair_per_tick: usize,
    aggression_permille: u16,
    amount_max: Decimal,
}

fn profile_config(profile: SimulatorProfile) -> ProfileConfig {
    match profile {
        SimulatorProfile::Normal => ProfileConfig {
            interval_ms: 550,
            orders_per_pair_per_tick: 3,
            aggression_permille: 450,
            amount_max: Decimal::new(800, 4),
        },
        SimulatorProfile::Fast => ProfileConfig {
            interval_ms: 250,
            orders_per_pair_per_tick: 8,
            aggression_permille: 580,
            amount_max: Decimal::new(1800, 4),
        },
        SimulatorProfile::Turbo => ProfileConfig {
            interval_ms: 120,
            orders_per_pair_per_tick: 16,
            aggression_permille: 700,
            amount_max: Decimal::new(3500, 4),
        },
        SimulatorProfile::Hyper => ProfileConfig {
            interval_ms: 70,
            orders_per_pair_per_tick: 28,
            aggression_permille: 800,
            amount_max: Decimal::new(5000, 4),
        },
    }
}

#[derive(Serialize)]
pub struct SimulatorStatusResponse {
    pub running: bool,
    pub profile: String,
    pub ticks: u64,
    pub total_orders: u64,
    pub total_fills: u64,
    pub pair_stats: HashMap<String, PairStatsDto>,
}

#[derive(Serialize)]
pub struct PairStatsDto {
    pub orders: u64,
    pub fills: u64,
}

#[derive(Deserialize)]
pub struct StartSimulatorRequest {
    pub profile: Option<String>,
}

#[derive(Deserialize)]
pub struct ProfileRequest {
    pub profile: String,
}

#[derive(Deserialize)]
struct PlaceOrderApiResponse {
    trades_count: usize,
}

#[derive(Deserialize)]
struct LiveTickerApiResponse {
    symbol: String,
    last_price: String,
}

#[derive(Serialize)]
pub struct SimulatorActionResponse {
    pub ok: bool,
    pub running: bool,
    pub profile: String,
}

pub fn spawn_simulator_worker(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut api_anchors: HashMap<String, Decimal> = HashMap::new();
        let mut last_anchor_refresh = Instant::now() - ANCHOR_REFRESH_INTERVAL;

        // On a cold start, try to prime anchors from live Binance tickers before
        // generating any synthetic orders. This avoids static seed prices.
        for _ in 0..10 {
            match fetch_live_anchors(&client).await {
                Ok(anchors) if !anchors.is_empty() => {
                    for (pair, price) in &anchors {
                        state.set_last_trade_price(pair.clone(), *price);
                    }
                    api_anchors = anchors;
                    break;
                }
                Ok(_) => {
                    tracing::warn!("simulator warm-up received empty live anchors");
                }
                Err(err) => {
                    tracing::warn!("simulator warm-up failed to fetch live anchors: {err}");
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        loop {
            let (running, profile) = {
                let sim = state.simulator.lock();
                (sim.running, sim.profile)
            };

            if !running {
                tokio::time::sleep(Duration::from_millis(400)).await;
                continue;
            }

            if last_anchor_refresh.elapsed() >= ANCHOR_REFRESH_INTERVAL {
                match fetch_live_anchors(&client).await {
                    Ok(anchors) if !anchors.is_empty() => {
                        for (pair, price) in &anchors {
                            state.set_last_trade_price(pair.clone(), *price);
                        }
                        api_anchors = anchors;
                    }
                    Ok(_) => {
                        tracing::warn!("simulator live ticker API returned no anchors");
                    }
                    Err(err) => {
                        tracing::warn!("simulator failed to fetch live ticker anchors: {err}");
                    }
                }
                last_anchor_refresh = Instant::now();
            }

            let config = profile_config(profile);
            if let Err(err) = run_one_tick(&state, &client, config, &api_anchors).await {
                tracing::warn!("simulator tick failed: {err}");
            }

            tokio::time::sleep(Duration::from_millis(config.interval_ms)).await;
        }
    })
}

async fn run_one_tick(
    state: &AppState,
    client: &reqwest::Client,
    config: ProfileConfig,
    api_anchors: &HashMap<String, Decimal>,
) -> Result<(), String> {
    let mut orders_delta = 0_u64;
    let mut fills_delta = 0_u64;
    let mut per_pair_delta: HashMap<String, SimulatorPairStats> = HashMap::new();

    for (pair, base, quote) in PAIRS {
        let Some(anchor) = api_anchors
            .get(pair)
            .copied()
            .or_else(|| state.get_last_trade_price(pair)) else {
            continue;
        };

        for _ in 0..config.orders_per_pair_per_tick {
            let is_buy = rand::thread_rng().gen_bool(0.5);
            let aggressive = rand::thread_rng().gen_range(0..1000) < u32::from(config.aggression_permille);

            let bps = if is_buy {
                if aggressive {
                    rand::thread_rng().gen_range(0..=15)
                } else {
                    -rand::thread_rng().gen_range(25..=125)
                }
            } else if aggressive {
                -rand::thread_rng().gen_range(0..=15)
            } else {
                rand::thread_rng().gen_range(25..=125)
            };

            let mut price = anchor + (anchor * Decimal::new(i64::from(bps), 4));
            if price < Decimal::new(1, 2) {
                price = Decimal::new(1, 2);
            }
            price = price.round_dp(2);

            let max_units = (config.amount_max * Decimal::new(10_000, 0))
                .trunc()
                .to_u64()
                .unwrap_or(10)
                .max(10);
            let amount_units = rand::thread_rng().gen_range(10..=max_units);
            let amount = Decimal::new(amount_units as i64, 4);

            let user_id = rand::thread_rng().gen_range(1..=4);
            let side = if is_buy { "buy" } else { "sell" };

            let resp = client
                .post(ORDER_ENDPOINT)
                .header("x-user-id", user_id.to_string())
                .json(&serde_json::json!({
                    "side": side,
                    "price": price.to_string(),
                    "amount": amount.to_string(),
                    "base_asset": base,
                    "quote_asset": quote,
                }))
                .send()
                .await
                .map_err(|e| format!("order request failed: {e}"))?;

            if !resp.status().is_success() {
                continue;
            }

            let body = resp
                .json::<PlaceOrderApiResponse>()
                .await
                .map_err(|e| format!("failed to parse order response: {e}"))?;

            orders_delta += 1;
            fills_delta += body.trades_count as u64;

            let stats = per_pair_delta
                .entry(pair.to_string())
                .or_insert(SimulatorPairStats { orders: 0, fills: 0 });
            stats.orders += 1;
            stats.fills += body.trades_count as u64;
        }
    }

    let mut sim = state.simulator.lock();
    sim.ticks += 1;
    sim.total_orders += orders_delta;
    sim.total_fills += fills_delta;

    for (pair, delta) in per_pair_delta {
        let entry = sim
            .pair_stats
            .entry(pair)
            .or_insert(SimulatorPairStats { orders: 0, fills: 0 });
        entry.orders += delta.orders;
        entry.fills += delta.fills;
    }

    Ok(())
}

async fn fetch_live_anchors(client: &reqwest::Client) -> Result<HashMap<String, Decimal>, String> {
    let response = client
        .get(LIVE_TICKERS_ENDPOINT)
        .send()
        .await
        .map_err(|e| format!("live ticker request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("live ticker endpoint returned status {}", response.status()));
    }

    let tickers = response
        .json::<Vec<LiveTickerApiResponse>>()
        .await
        .map_err(|e| format!("invalid live ticker response: {e}"))?;

    let mut out = HashMap::new();
    for ticker in tickers {
        let pair = ticker_symbol_to_pair(&ticker.symbol);
        let Some(pair_symbol) = pair else {
            continue;
        };

        let Ok(last_price) = Decimal::from_str(ticker.last_price.trim()) else {
            continue;
        };

        if !last_price.is_sign_negative() && !last_price.is_zero() {
            out.insert(pair_symbol.to_string(), last_price);
        }
    }

    Ok(out)
}

fn ticker_symbol_to_pair(symbol: &str) -> Option<&'static str> {
    match symbol {
        "BTCUSDT" => Some("BTC_USDT"),
        "ETHUSDT" => Some("ETH_USDT"),
        "SOLUSDT" => Some("SOL_USDT"),
        "BNBUSDT" => Some("BNB_USDT"),
        _ => None,
    }
}

pub async fn simulator_status_handler(
    State(state): State<AppState>,
) -> Json<SimulatorStatusResponse> {
    Json(simulator_status_from_state(&state))
}

pub async fn simulator_start_handler(
    State(state): State<AppState>,
    Json(payload): Json<StartSimulatorRequest>,
) -> Result<Json<SimulatorActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut sim = state.simulator.lock();

    if let Some(profile_text) = payload.profile.as_deref() {
        let profile = SimulatorProfile::parse(profile_text).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid simulator profile" })),
            )
        })?;
        sim.profile = profile;
    }

    sim.running = true;

    Ok(Json(SimulatorActionResponse {
        ok: true,
        running: sim.running,
        profile: sim.profile.as_str().to_string(),
    }))
}

pub async fn simulator_stop_handler(
    State(state): State<AppState>,
) -> Json<SimulatorActionResponse> {
    let mut sim = state.simulator.lock();
    sim.running = false;

    Json(SimulatorActionResponse {
        ok: true,
        running: sim.running,
        profile: sim.profile.as_str().to_string(),
    })
}

pub async fn simulator_reset_handler(
    State(state): State<AppState>,
) -> Json<SimulatorActionResponse> {
    let mut sim = state.simulator.lock();
    sim.reset_counters();

    Json(SimulatorActionResponse {
        ok: true,
        running: sim.running,
        profile: sim.profile.as_str().to_string(),
    })
}

pub async fn simulator_profile_handler(
    State(state): State<AppState>,
    Json(payload): Json<ProfileRequest>,
) -> Result<Json<SimulatorActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut sim = state.simulator.lock();
    let profile = SimulatorProfile::parse(payload.profile.as_str()).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid simulator profile" })),
        )
    })?;

    sim.profile = profile;

    Ok(Json(SimulatorActionResponse {
        ok: true,
        running: sim.running,
        profile: sim.profile.as_str().to_string(),
    }))
}

fn simulator_status_from_state(state: &AppState) -> SimulatorStatusResponse {
    let sim = state.simulator.lock();
    let pair_stats = sim
        .pair_stats
        .iter()
        .map(|(pair, stats)| {
            (
                pair.clone(),
                PairStatsDto {
                    orders: stats.orders,
                    fills: stats.fills,
                },
            )
        })
        .collect();

    SimulatorStatusResponse {
        running: sim.running,
        profile: sim.profile.as_str().to_string(),
        ticks: sim.ticks,
        total_orders: sim.total_orders,
        total_fills: sim.total_fills,
        pair_stats,
    }
}
