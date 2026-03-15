# doan2_DH22KPM02_VoNguyenGiaHuy_MatchingEngine_ZKP

## OPS Runtime (Docker Compose)

This repository now includes a full local runtime stack:

- `db`: PostgreSQL 17 (`cex_postgres`)
- `backend`: Rust Axum service (`cex_backend`)
- `web`: Nginx reverse proxy + static SPA (`cex_web`)

### Start all services

```powershell
docker compose up -d --build
docker compose ps
```

### Routing model

- Browser entrypoint: `http://localhost:8080`
- Nginx proxies:
	- `/health` -> backend `/health`
	- `/api/*` -> backend `/api/*`
	- `/ws` -> backend websocket endpoint `/ws`

### Quick verification

```powershell
Invoke-RestMethod http://localhost:8080/health
Invoke-RestMethod http://localhost:8080/api/orderbook
```

Expected:

- `/health` returns `{ "status": "ok" }`
- `/api/orderbook` returns `{ "bids": [], "asks": [] }` on empty book

### Stop services

```powershell
docker compose down
```