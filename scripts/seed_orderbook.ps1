# =============================================================================
# scripts/seed_orderbook.ps1
# Seed the matching engine with a Binance BTC/USDT orderbook snapshot.
#
# Usage (run from project root):
#   .\scripts\seed_orderbook.ps1 [-Api "http://localhost:3000"]
#
# Requirements:
#   - Matching engine running (cargo run -p core or docker compose up)
#   - data\btcusdt_snapshot.json present
#
# What this script does:
#   1. Posts all ASK levels as SELL orders from alice  (user 1)
#   2. Posts all BID levels as BUY  orders from bob    (user 2)
#      Asks > Bids, so no crossing happens: clean order book
#   3. Simulates 2 aggressive crossing orders (charlie/dave) to generate trades
# =============================================================================
param(
    [string]$Api = "http://localhost:3000"
)
$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"

$Snapshot = "data\btcusdt_snapshot.json"

# ── Pre-flight checks ─────────────────────────────────────────────────────────
if (-not (Test-Path $Snapshot)) {
    Write-Error "File not found: $Snapshot — run from project root."
    exit 1
}

Write-Host "Checking server at $Api ..."
try {
    $health = Invoke-RestMethod -Uri "$Api/health" -Method Get -TimeoutSec 5
    Write-Host "Server OK: $($health | ConvertTo-Json -Compress)"
} catch {
    Write-Error "Server is not responding at $Api`nStart locally: cargo run -p core`nOr via Docker: docker compose up -d"
    exit 1
}
Write-Host ""

# ── Helper function ───────────────────────────────────────────────────────────
function Post-Order {
    param (
        [int]    $UserId,
        [string] $Side,
        [string] $Price,
        [string] $Amount
    )
    $body = @{
        side        = $Side
        price       = $Price
        amount      = $Amount
        base_asset  = "BTC"
        quote_asset = "USDT"
    } | ConvertTo-Json -Compress

    try {
        return Invoke-RestMethod `
            -Uri     "$Api/api/orders" `
            -Method  Post `
            -Headers @{ "x-user-id" = "$UserId"; "Content-Type" = "application/json" } `
            -Body    $body `
            -TimeoutSec 10
    } catch {
        return @{ error = $_.ToString() }
    }
}

# ── Load snapshot ─────────────────────────────────────────────────────────────
$snap = Get-Content $Snapshot -Raw | ConvertFrom-Json
$fetchedAt = if ($snap.fetched_at) { $snap.fetched_at } else { "static file" }

Write-Host "Dataset : $fetchedAt"
Write-Host "Source  : $($snap.source)"
Write-Host "Best bid: `$$($snap.bids[0][0])  |  Best ask: `$$($snap.asks[0][0])"
Write-Host "Levels  : $($snap.bids.Count) bids, $($snap.asks.Count) asks"
Write-Host ""

# ── Step 1: Post asks (sell orders) from alice ───────────────────────────────
Write-Host "[1/3] Posting $($snap.asks.Count) SELL orders  (alice, user 1) ..."
$okAsks = 0
foreach ($entry in $snap.asks) {
    $price  = $entry[0]
    $amount = $entry[1]
    $res    = Post-Order -UserId 1 -Side "sell" -Price $price -Amount $amount
    if ($res.order_id) {
        Write-Host ("  SELL {0} BTC @ `${1,-12}  order_id={2}" -f $amount, $price, $res.order_id)
        $okAsks++
    } else {
        Write-Warning "  FAIL @ `$$price : $($res.error)"
    }
}
Write-Host "  -> $okAsks/$($snap.asks.Count) sell orders placed."
Write-Host ""

# ── Step 2: Post bids (buy orders) from bob ──────────────────────────────────
Write-Host "[2/3] Posting $($snap.bids.Count) BUY  orders  (bob, user 2) ..."
$okBids = 0
foreach ($entry in $snap.bids) {
    $price  = $entry[0]
    $amount = $entry[1]
    $res    = Post-Order -UserId 2 -Side "buy" -Price $price -Amount $amount
    if ($res.order_id) {
        Write-Host ("  BUY  {0} BTC @ `${1,-12}  order_id={2}" -f $amount, $price, $res.order_id)
        $okBids++
    } else {
        Write-Warning "  FAIL @ `$$price : $($res.error)"
    }
}
Write-Host "  -> $okBids/$($snap.bids.Count) buy orders placed."
Write-Host ""

# ── Step 3: Simulate crossing orders to create real trades ───────────────────
Write-Host "[3/3] Simulating 2 market crosses to generate trade events ..."

# charlie buys aggressively above best ask -> fills against alice's sells
$r1 = Post-Order -UserId 3 -Side "buy" -Price "65003.00" -Amount "2.00000000"
Write-Host ("  charlie BUY  2.00 BTC @ `$65003.00  trades={0}" -f $r1.trades_count)

# dave sells aggressively below best bid -> fills against bob's buys
$r2 = Post-Order -UserId 4 -Side "sell" -Price "64998.00" -Amount "2.00000000"
Write-Host ("  dave   SELL 2.00 BTC @ `$64998.00  trades={0}" -f $r2.trades_count)

Write-Host ""
Write-Host ("=" * 60)
Write-Host "Orderbook seeded successfully!"
Write-Host "  View snapshot:   curl $Api/api/orderbook"
$wsUri = $Api -replace "^http://", ""
Write-Host "  WebSocket feed:  ws://$wsUri/ws"
Write-Host "  ZK proof (u1):   curl -H 'x-user-id: 1' $Api/api/zkp/proof?asset=USDT"
Write-Host "  Frontend:        http://localhost:8080"
Write-Host ("=" * 60)
