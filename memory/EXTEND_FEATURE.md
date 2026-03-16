# Enterprise CEX Architecture: The 6 Core Pillars

This document outlines the complete lifecycle and architectural pillars of a production-grade Centralized Exchange (CEX).

## Pillar 1: Identity & Access Management (Simple Auth)
* **Registration & Login:** Streamlined account creation using only a **Username and Password**. No email verification is required, keeping the flow fast and simple for testing and demonstration.
* **Security:** Passwords are hashed in the database using **Argon2id**. 
* **Session Management:** Successful logins return a standard **JSON Web Token (JWT)**. Clients send this token via the `Authorization: Bearer <token>` header.
* **Middleware:** An `axum::middleware` layer decodes the JWT, extracts the `user_id`, and safely passes it into the engine's core.

## Pillar 2: In-Memory Ledger (Stateful Wallets)
* **Balance Segregation:** Each user's asset (e.g., USDT, BTC) is split into two states within the RAM: `Free Balance` and `Locked Balance`.
* **State Transition:** Placing an order deducts from `Free` and adds to `Locked`. Canceling an order reverses this atomically.
* **Internal Hashing:** All internal ID generation utilizes **BLAKE3** for maximum speed.

## Pillar 3: High-Frequency Matching Engine (The Core)
* **Data Structure:** Implements `BTreeMap` for deterministic, `O(log n)` order insertion and retrieval.
  * `Bids` (Buyers): Sorted descending (highest price first).
  * `Asks` (Sellers): Sorted ascending (lowest price first).
* **Execution Logic:** Strictly enforces **Price-Time Priority**. All math uses `rust_decimal` to prevent floating-point precision loss. Enforces Self-Trading Prevention (STP) to reject orders where taker and maker share the same `user_id`.

## Pillar 4: Real-Time Gateway (Pub/Sub)
* **REST API:** Handles state-mutating actions (Place Order, Cancel Order, Deposit).
* **WebSocket Streams (`/ws`):** A high-throughput broadcast system sending `OrderBookUpdate` and `RecentTrade` events to maintain live UI depth charts.

## Pillar 5: Settlement & Asynchronous Persistence
* **RAM-First Settlement:** Upon an engine match, balances are updated directly in memory to maintain micro-second latency.
* **Database Flushing:** Events pass through a lock-free `tokio::sync::mpsc` channel. A background worker batches these events and writes them to PostgreSQL via `sqlx`.

## Pillar 6: Observability & Proof of Solvency
* **Telemetry:** Uses the `metrics` crate with atomic counters (~1ns overhead) to expose Prometheus endpoints for monitoring in Grafana.
* **ZKP Solvency:** Constructs a Merkle Sum Tree from the database state. The frontend executes a WebAssembly (Wasm) verifier to mathematically prove `Total Liabilities <= Cold Wallet Assets`.