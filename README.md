# CEX ZKP

High-performance centralized exchange simulation with an in-memory Rust matching engine, PostgreSQL async persistence, and ZK Proof of Solvency support.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Repository Structure](#repository-structure)
- [Core Features](#core-features)
- [Tech Stack](#tech-stack)
- [Quick Start (Docker Compose)](#quick-start-docker-compose)
- [Run in Local Development Mode](#run-in-local-development-mode)
- [API Overview](#api-overview)
- [WebSocket Feed](#websocket-feed)
- [Testing and Benchmark](#testing-and-benchmark)
- [Observability](#observability)
- [Roadmap Status](#roadmap-status)
- [Contributing](#contributing)

## Overview

This project focuses on three pillars:

- Deterministic in-memory order matching with low latency.
- Durable asynchronous persistence to PostgreSQL.
- Verifiable exchange solvency via Merkle Sum Tree and ZK proof tooling.

The backend runs as a monolith in Rust. Matching logic stays in RAM while database writes happen asynchronously in the background.

## Architecture

```text
Browser (Svelte SPA)
	-> Nginx (reverse proxy, static host)
		-> Rust Axum Backend (REST + WS + matching engine)
			-> In-memory Order Book (BTreeMap)
			-> Async persistence worker (sqlx -> PostgreSQL)
			-> ZKP module (Merkle Sum Tree + proof package)
```

Runtime services in Docker Compose:

- `db`: PostgreSQL 17 (`cex_postgres`)
- `backend`: Rust Axum API (`cex_backend`)
- `web`: Nginx + built SPA (`cex_web`)
- `prometheus`: metrics scraping (`cex_prometheus`)
- `grafana`: dashboards (`cex_grafana`)

## Repository Structure

```text
.
|- core/      # Rust backend: engine, API, DB, observability, benchmarks
|- web/       # Svelte + Vite SPA
|- zkp/       # Merkle Sum Tree + ZK primitives + wasm bindings
|- docs/      # Technical docs, architecture spec, WBS
|- docker/    # Nginx, Prometheus, Grafana provisioning
`- docker-compose.yml
```

## Core Features

- In-memory order matching engine using `BTreeMap` price levels.
- REST APIs for orders, balances, market data, wallet flows, and admin operations.
- Real-time WebSocket market feed at `/ws`.
- Async background persistence worker for orders/trades/balance snapshots.
- Solvency endpoints:
	- `GET /api/zkp/proof`
	- `GET /api/zkp/solvency`
- Built-in observability:
	- `GET /health`
	- `GET /metrics`

## Tech Stack

- Backend: Rust, Tokio, Axum
- Numeric precision: `rust_decimal` (no floating-point for money)
- Data persistence: PostgreSQL + `sqlx`
- Frontend: Svelte 5 + Vite + Tailwind CSS
- ZKP: arkworks ecosystem + Poseidon hash
- Infra: Docker Compose + Nginx + Prometheus + Grafana

## Quick Start (Docker Compose)

### 1) Prepare environment

```powershell
# First time only
Copy-Item .env.example .env

# Optional: configure Binance credentials for live ticker proxy
# BINANCE_API_BASE_URL=https://api.binance.com/api/v3
# BINANCE_API_KEY=...
# BINANCE_API_SECRET=...
```

Notes:

- Keep `.env` private and never commit it.
- Frontend consumes live market data through backend endpoint `/api/market/tickers/live`.

### 2) Build and run

```powershell
# Recommended when migration files changed
docker compose down -v

docker compose up -d --build
docker compose ps
```

### 3) Access services

- App: `http://localhost:8080`
- Backend direct: `http://localhost:3000`
- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3001` (`admin` / `admin`)

### 4) Quick verification

```powershell
Invoke-RestMethod http://localhost:8080/health
Invoke-RestMethod "http://localhost:8080/api/orderbook?symbol=BTC_USDT"
Invoke-WebRequest http://localhost:8080/metrics -UseBasicParsing | Select-Object -ExpandProperty StatusCode
```

Expected:

- `/health` returns `{ "status": "ok" }`
- `/api/orderbook` returns a snapshot JSON with `bids` and `asks`
- `/metrics` returns HTTP `200`

### 5) Stop runtime

```powershell
docker compose down
```

## Run in Local Development Mode

### Backend

```powershell
cargo run -p core --bin core
```

### Frontend

```powershell
cd web
npm install
npm run dev
```

### ZKP crate tests

```powershell
cargo test -p zkp
```

## API Overview

Public/system endpoints:

- `GET /health`
- `GET /metrics`
- `GET /api/orderbook`
- `GET /api/assets`
- `GET /api/market/tickers/live`
- `GET /api/price/average`
- `GET /api/trades/recent`
- `GET /api/candles`
- `GET /api/zkp/solvency`
- `GET /ws`

Auth and user endpoints:

- `POST /api/auth/register`
- `POST /api/auth/login`
- `GET /api/auth/me`
- `GET /api/auth/users`
- `PUT /api/auth/display-name`

Trading and wallet endpoints:

- `POST /api/orders`
- `DELETE /api/orders/:id`
- `GET /api/orders/open`
- `GET /api/balances`
- `GET /api/balances/:asset`
- `POST /api/deposit`
- `POST /api/withdraw`
- `POST /api/transfer`
- `GET /api/trades/user`

Simulation and admin endpoints:

- `GET /api/simulator/status`
- `POST /api/simulator/start`
- `POST /api/simulator/stop`
- `POST /api/simulator/reset`
- `PUT /api/simulator/profile`
- `GET /api/admin/metrics`
- `GET /api/admin/treasury`
- `POST /api/admin/treasury/deposit`
- `POST /api/admin/treasury/withdraw`
- `GET /api/admin/assets`
- `POST /api/admin/assets`
- `POST /api/admin/markets/halt`
- `GET /api/admin/users`
- `PUT /api/admin/users/:id/suspend`
- `POST /api/admin/zkp/snapshot`
- `GET /api/admin/zkp/history`

### Authentication model in development

Development and simulation flows support a lightweight user identity model via `x-user-id` header for user-scoped actions.

## WebSocket Feed

- Endpoint: `GET /ws`
- Purpose: stream real-time orderbook and trade updates to clients.
- Transport: native WebSocket upgrade through Nginx.

## Testing and Benchmark

From repository root:

```powershell
# Full workspace tests
cargo test

# Engine property tests and unit tests
cargo test -p core

# Matching benchmark (Criterion)
cargo bench -p core --bench engine_benchmark
```

## Observability

- Prometheus scrapes backend metrics from `backend:3000/metrics`.
- Grafana includes a pre-provisioned dashboard: `CEX Observability`.
- Backend health check is available at `/health` and used by Docker Compose.

