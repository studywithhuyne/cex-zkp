#!/usr/bin/env bash
# =============================================================================
# scripts/seed_orderbook.sh
# Seed the matching engine with a Binance BTC/USDT orderbook snapshot.
#
# Usage (run from project root):
#   bash scripts/seed_orderbook.sh [API_URL]
#
# Requirements:
#   - Matching engine running (cargo run -p core or docker compose up)
#   - python3 (stdlib only, no pip installs needed)
#   - data/btcusdt_snapshot.json present
#
# What this script does:
#   1. Posts all ASK levels as SELL orders from alice  (user 1)
#   2. Posts all BID levels as BUY  orders from bob    (user 2)
#      → Asks > Bids, so no crossing happens: clean order book
#   3. Simulates 2 aggressive crossing orders (charlie/dave) to generate trades
# =============================================================================
set -euo pipefail

API="${1:-http://localhost:3000}"
SNAPSHOT="data/btcusdt_snapshot.json"

# ── Pre-flight checks ─────────────────────────────────────────────────────────
if [ ! -f "$SNAPSHOT" ]; then
  echo "ERROR: $SNAPSHOT not found. Run from project root." >&2
  exit 1
fi

if ! command -v python3 &>/dev/null; then
  echo "ERROR: python3 is required." >&2
  exit 1
fi

echo "Checking server at $API ..."
if ! curl -sf "$API/health" >/dev/null; then
  echo ""
  echo "ERROR: Server is not responding at $API"
  echo "  Start locally: cargo run -p core"
  echo "  Or via Docker: docker compose up -d"
  exit 1
fi
echo "Server OK."
echo ""

# ── Seed + simulate with Python ───────────────────────────────────────────────
python3 <<PYEOF
import json, urllib.request, urllib.error, sys

API = "${API}"
SNAPSHOT = "${SNAPSHOT}"

HEADERS = {"Content-Type": "application/json"}

def post_order(user_id, side, price, amount):
    payload = json.dumps({
        "side":        side,
        "price":       price,
        "amount":      amount,
        "base_asset":  "BTC",
        "quote_asset": "USDT",
    }).encode()
    req = urllib.request.Request(
        f"{API}/api/orders",
        data=payload,
        headers={**HEADERS, "x-user-id": str(user_id)},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return json.loads(r.read())
    except urllib.error.HTTPError as e:
        return {"error": f"HTTP {e.code}: {e.read().decode()}"}
    except Exception as e:
        return {"error": str(e)}

with open(SNAPSHOT) as f:
    snap = json.load(f)

print(f"Dataset: {snap.get('fetched_at', 'static file')}  |  source: {snap.get('source', 'N/A')}")
print(f"Best bid: \${snap['bids'][0][0]}  |  Best ask: \${snap['asks'][0][0]}")
print(f"Levels: {len(snap['bids'])} bids, {len(snap['asks'])} asks")
print()

# ── Step 1: Post asks (sell orders) from alice ───────────────────────────────
ok_asks = 0
print(f"[1/3] Posting {len(snap['asks'])} SELL orders  (alice, user 1) ...")
for entry in snap["asks"]:
    price, amount = entry[0], entry[1]
    res = post_order(1, "sell", price, amount)
    if "order_id" in res:
        print(f"  SELL {amount} BTC @ \${price:<12}  order_id={res['order_id']}")
        ok_asks += 1
    else:
        print(f"  FAIL @ \${price}: {res.get('error', res)}", file=sys.stderr)
print(f"  -> {ok_asks}/{len(snap['asks'])} sell orders placed.")
print()

# ── Step 2: Post bids (buy orders) from bob ──────────────────────────────────
ok_bids = 0
print(f"[2/3] Posting {len(snap['bids'])} BUY  orders  (bob, user 2) ...")
for entry in snap["bids"]:
    price, amount = entry[0], entry[1]
    res = post_order(2, "buy", price, amount)
    if "order_id" in res:
        print(f"  BUY  {amount} BTC @ \${price:<12}  order_id={res['order_id']}")
        ok_bids += 1
    else:
        print(f"  FAIL @ \${price}: {res.get('error', res)}", file=sys.stderr)
print(f"  -> {ok_bids}/{len(snap['bids'])} buy orders placed.")
print()

# ── Step 3: Simulate crossing orders to create real trades ───────────────────
print("[3/3] Simulating 2 market crosses to generate trade events ...")

# charlie buys aggressively above best ask → fills against alice's sells
r1 = post_order(3, "buy", "65003.00", "2.00000000")
trades1 = r1.get("trades_count", "?")
print(f"  charlie BUY  2.00 BTC @ \$65003.00  trades={trades1}  {r1}")

# dave sells aggressively below best bid → fills against bob's buys
r2 = post_order(4, "sell", "64998.00", "2.00000000")
trades2 = r2.get("trades_count", "?")
print(f"  dave   SELL 2.00 BTC @ \$64998.00  trades={trades2}  {r2}")

print()
print("=" * 60)
print("Orderbook seeded successfully!")
print(f"  View snapshot:   curl {API}/api/orderbook")
print(f"  WebSocket feed:  ws://{API.replace('http://', '')}/ws")
print(f"  ZK proof (u1):   curl -H 'x-user-id: 1' {API}/api/zkp/proof?asset=USDT")
print(f"  Frontend:        http://localhost:8080")
print("=" * 60)
PYEOF
