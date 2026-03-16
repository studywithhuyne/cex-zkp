#!/usr/bin/env bash
# =============================================================================
# scripts/fetch_binance.sh
# Fetch a fresh BTC/USDT orderbook snapshot + recent trades from Binance
# and overwrite the static files in data/.
#
# Usage (run from project root):
#   bash scripts/fetch_binance.sh [SYMBOL] [DEPTH_LIMIT] [TRADES_LIMIT]
#
# Examples:
#   bash scripts/fetch_binance.sh              # BTC/USDT, 15 levels, 20 trades
#   bash scripts/fetch_binance.sh ETHUSDT      # ETH/USDT
#   bash scripts/fetch_binance.sh BTCUSDT 30 50
#
# Requirements:
#   - python3 (stdlib only: json, urllib.request)
#   - Internet access to api.binance.com (public endpoints, no API key needed)
# =============================================================================
set -euo pipefail

SYMBOL="${1:-BTCUSDT}"
DEPTH_LIMIT="${2:-15}"
TRADES_LIMIT="${3:-20}"

if ! command -v python3 &>/dev/null; then
  echo "ERROR: python3 is required." >&2
  exit 1
fi

mkdir -p data

echo "Fetching Binance $SYMBOL data  (depth=$DEPTH_LIMIT, trades=$TRADES_LIMIT) ..."

python3 <<PYEOF
import json, urllib.request, urllib.error, datetime, sys

SYMBOL       = "${SYMBOL}"
DEPTH_LIMIT  = ${DEPTH_LIMIT}
TRADES_LIMIT = ${TRADES_LIMIT}
BASE         = "https://api.binance.com/api/v3"
TIMEOUT      = 10

def fetch(url):
    try:
        with urllib.request.urlopen(url, timeout=TIMEOUT) as r:
            return json.loads(r.read())
    except urllib.error.URLError as e:
        print(f"Network error: {e}", file=sys.stderr)
        sys.exit(1)

now = datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")

# ── Orderbook depth snapshot ─────────────────────────────────────────────────
print(f"  GET /api/v3/depth?symbol={SYMBOL}&limit={DEPTH_LIMIT} ...", end=" ", flush=True)
depth_url = f"{BASE}/depth?symbol={SYMBOL}&limit={DEPTH_LIMIT}"
raw_depth = fetch(depth_url)

snapshot = {
    "symbol":       SYMBOL,
    "source":       f"Binance REST API — /api/v3/depth?symbol={SYMBOL}&limit={DEPTH_LIMIT}",
    "fetched_at":   now,
    "lastUpdateId": raw_depth["lastUpdateId"],
    "bids":         raw_depth["bids"],
    "asks":         raw_depth["asks"],
}
with open("data/btcusdt_snapshot.json", "w") as f:
    json.dump(snapshot, f, indent=2)

best_bid = raw_depth["bids"][0][0]  if raw_depth["bids"] else "N/A"
best_ask = raw_depth["asks"][0][0]  if raw_depth["asks"] else "N/A"
spread   = float(best_ask) - float(best_bid)
print(f"OK  bid=\${best_bid}  ask=\${best_ask}  spread=\${spread:.2f}")

# ── Aggregate trades ─────────────────────────────────────────────────────────
print(f"  GET /api/v3/aggTrades?symbol={SYMBOL}&limit={TRADES_LIMIT} ...", end=" ", flush=True)
trades_url = f"{BASE}/aggTrades?symbol={SYMBOL}&limit={TRADES_LIMIT}"
trades = fetch(trades_url)
with open("data/btcusdt_trades.json", "w") as f:
    json.dump(trades, f, indent=2)
print(f"OK  {len(trades)} trades  latest=\${trades[-1]['p']}")

# ── Summary ───────────────────────────────────────────────────────────────────
total_bid_vol = sum(float(b[1]) for b in raw_depth["bids"])
total_ask_vol = sum(float(a[1]) for a in raw_depth["asks"])
print()
print(f"Saved data/btcusdt_snapshot.json  ({len(raw_depth['bids'])} bids, {len(raw_depth['asks'])} asks)")
print(f"  Total bid volume: {total_bid_vol:.5f} {SYMBOL.replace('USDT','')}  |  ask volume: {total_ask_vol:.5f} {SYMBOL.replace('USDT','')}")
print(f"Saved data/btcusdt_trades.json   ({len(trades)} aggTrades)")
print()
print("Run seeds:  bash scripts/seed_orderbook.sh")
PYEOF
