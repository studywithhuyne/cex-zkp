# Comprehensive CEX Architecture & Feature Specifications

This document outlines the architecture, frontend routing, and enterprise-grade backend features for the High-Performance CEX. All backend implementations strictly prioritize micro-second latency matching.

## 1. Frontend Architecture & Routing
* **App Shell & Deterministic Identity:**
  * Persistent Navbar linking to `/trade`, `/wallet`, and `/zk-verify`.
  * **Mock Auth Module:** A dropdown to select dummy users (e.g., "Alice - ID: 1"). This automatically injects the `x-user-id` HTTP header (parsed as `u64`) into all REST and WebSocket requests, bypassing JWT/OAuth overhead during critical engine paths.
* **Trading Terminal (`/trade`):**
  * **OrderBook:** Real-time visual depth chart mapped via WebSocket streams.
  * **Trade Form:** Inputs for Price (`rust_decimal` compatible) and Amount.
  * **Order Management:** Unexecuted open orders list with instant "Cancel" actions, plus a live ticker for recent market trades.
* **Portfolio & Wallet (`/wallet`):**
  * Real-time display of Base and Quote token balances.
  * **Mock Deposit:** Form to inject test funds directly into PostgreSQL.
  * Personal executed trade history table.

## 2. Core Engine & Data Flow
* **Multi-Symbol Routing (O(1) Complexity):** The core Engine utilizes a `HashMap<String, OrderBook>` to route orders to specific trading pairs (e.g., `BTC_USDT`, `ETH_USDT`) instantly. Both `Order` and `Trade` structs strictly enforce a `symbol` field.
* **Real-Time Price Tracking (OHLCV):** * The matching engine NEVER calculates chart data directly to preserve latency.
  * Upon a match, a `Trade` event is pushed to a lock-free async channel (`tokio::sync::mpsc`).
  * A background worker consumes this channel, flushes data to PostgreSQL, and Axum serves the candlestick data to the Svelte frontend.

## 3. Zero-Knowledge Proof of Solvency (ZKP)
* **Core Philosophy:** "Don't trust, verify." 
* **Backend Mechanism (Merkle Sum Tree):** * Each leaf represents a user's `Hash(x-user-id, balance)` and their `balance`.
  * The root node encapsulates the sum of all liabilities.
  * The ZK Circuit (`arkworks` or `halo2`) enforces: `Total Liabilities <= Cold Wallet Assets`.
* **Frontend Verification (`/zk-verify`):** * Retrieves the `Merkle Root` and user's `.proof` payload from the Axum API.
  * Provides a drag-and-drop UI for the proof file.
  * Executes the cryptographic verifier compiled to WebAssembly (Wasm) directly within the browser CPU, outputting a deterministic `VALID` or `INVALID` result.

## 4. Sub-millisecond Observability (Prometheus & Grafana)
* **Metric Collection:** Utilizes the Rust `metrics` crate with atomic counters. Incrementing a metric takes ~1-2 nanoseconds, keeping the engine's speed uncompromised.
* **Exporting & Visualization:** * Axum exposes a lightweight `GET /metrics` endpoint.
  * A Dockerized **Prometheus** instance scrapes data every 1 second.
  * **Grafana** visualizes critical metrics: Throughput (Orders/sec), Latency percentiles (p50, p90, p99), active symbols, and total locked value.