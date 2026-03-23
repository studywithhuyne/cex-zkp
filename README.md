# doan2_DH22KPM02_VoNguyenGiaHuy_MatchingEngine_ZKP

## OPS Runtime (Docker Compose)

### Environment setup

```powershell
# First time only
Copy-Item .env.example .env

# Then edit .env and set your real Binance credentials
# BINANCE_API_KEY=...
# BINANCE_API_SECRET=...
```

Notes:

- `.env` is ignored by git and must not be pushed.
- Frontend does not call Binance directly anymore; it consumes backend proxy endpoint `/api/market/tickers/live`.
- Binance credentials are read server-side from env (`BINANCE_API_BASE_URL`, `BINANCE_API_KEY`, `BINANCE_API_SECRET`).

This repository now includes a full local runtime stack:

- `db`: PostgreSQL 17 (`cex_postgres`)
- `backend`: Rust Axum service (`cex_backend`)
- `web`: Nginx reverse proxy + static SPA (`cex_web`)

### Start all services

```powershell
# If migration files changed (especially 0001), reset persisted DB metadata first
docker compose down -v
docker compose up -d --build
docker compose ps
```

### Routing model

- Browser entrypoint: `http://localhost:8080`
- Nginx proxies:
	- `/health` -> backend `/health`
	- `/metrics` -> backend `/metrics`
	- `/api/*` -> backend `/api/*`
	- `/ws` -> backend websocket endpoint `/ws`

### Observability

- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3001` (default login: `admin` / `admin`)
- Pre-provisioned dashboard: `CEX Observability`

Prometheus scrapes backend metrics every 1 second from `backend:3000/metrics`.

### Quick verification

```powershell
Invoke-RestMethod http://localhost:8080/health
Invoke-RestMethod http://localhost:8080/api/orderbook
Invoke-WebRequest http://localhost:8080/metrics -UseBasicParsing | Select-Object -ExpandProperty StatusCode
```

Expected:

- `/health` returns `{ "status": "ok" }`
- `/api/orderbook` returns `{ "bids": [], "asks": [] }` on empty book
- `/metrics` returns HTTP `200` and Prometheus text exposition

### Stop services

```powershell
docker compose down
```