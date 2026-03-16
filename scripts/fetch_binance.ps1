# =============================================================================
# scripts/fetch_binance.ps1
# Fetch a fresh BTC/USDT orderbook snapshot + recent trades from Binance
# and overwrite the static files in data\.
#
# Usage (run from project root):
#   .\scripts\fetch_binance.ps1 [-Symbol BTCUSDT] [-DepthLimit 15] [-TradesLimit 20]
#
# Examples:
#   .\scripts\fetch_binance.ps1
#   .\scripts\fetch_binance.ps1 -Symbol ETHUSDT
#   .\scripts\fetch_binance.ps1 -Symbol BTCUSDT -DepthLimit 30 -TradesLimit 50
#
# Requirements:
#   - Internet access to api.binance.com (public endpoints, no API key needed)
# =============================================================================
param(
    [string] $Symbol      = "BTCUSDT",
    [int]    $DepthLimit  = 15,
    [int]    $TradesLimit = 20
)
$ErrorActionPreference  = "Stop"
$ProgressPreference     = "SilentlyContinue"

$BaseUrl = "https://api.binance.com/api/v3"

if (-not (Test-Path "data")) {
    New-Item -ItemType Directory -Path "data" | Out-Null
}

Write-Host "Fetching Binance $Symbol data  (depth=$DepthLimit, trades=$TradesLimit) ..."

# ── Orderbook depth snapshot ─────────────────────────────────────────────────
$depthUrl = "$BaseUrl/depth?symbol=$Symbol&limit=$DepthLimit"
Write-Host "  GET $depthUrl ..." -NoNewline

$rawDepth = Invoke-RestMethod -Uri $depthUrl -Method Get -TimeoutSec 10

$now = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$snapshot = [ordered]@{
    symbol       = $Symbol
    source       = "Binance REST API — /api/v3/depth?symbol=$Symbol&limit=$DepthLimit"
    fetched_at   = $now
    lastUpdateId = $rawDepth.lastUpdateId
    bids         = $rawDepth.bids
    asks         = $rawDepth.asks
}

$snapshot | ConvertTo-Json -Depth 10 | Set-Content "data\btcusdt_snapshot.json" -Encoding UTF8

$bestBid = $rawDepth.bids[0][0]
$bestAsk = $rawDepth.asks[0][0]
$spread  = [math]::Round([double]$bestAsk - [double]$bestBid, 2)
Write-Host ("  OK  bid=`${0}  ask=`${1}  spread=`${2}" -f $bestBid, $bestAsk, $spread)

# ── Aggregate trades ─────────────────────────────────────────────────────────
$tradesUrl = "$BaseUrl/aggTrades?symbol=$Symbol&limit=$TradesLimit"
Write-Host "  GET $tradesUrl ..." -NoNewline

$trades = Invoke-RestMethod -Uri $tradesUrl -Method Get -TimeoutSec 10
$trades | ConvertTo-Json -Depth 10 | Set-Content "data\btcusdt_trades.json" -Encoding UTF8

$latestPrice = $trades[-1].p
Write-Host ("  OK  {0} trades  latest=`${1}" -f $trades.Count, $latestPrice)

# ── Summary ───────────────────────────────────────────────────────────────────
$totalBidVol = ($rawDepth.bids | ForEach-Object { [double]$_[1] } | Measure-Object -Sum).Sum
$totalAskVol = ($rawDepth.asks | ForEach-Object { [double]$_[1] } | Measure-Object -Sum).Sum
$baseAsset   = $Symbol -replace "USDT", ""

Write-Host ""
Write-Host ("Saved data\btcusdt_snapshot.json  ({0} bids, {1} asks)" -f $rawDepth.bids.Count, $rawDepth.asks.Count)
Write-Host ("  Total bid volume: {0:F5} {1}  |  ask volume: {2:F5} {1}" -f $totalBidVol, $baseAsset, $totalAskVol)
Write-Host ("Saved data\btcusdt_trades.json    ({0} aggTrades)" -f $trades.Count)
Write-Host ""
Write-Host "Run seeds:  .\scripts\seed_orderbook.ps1"
