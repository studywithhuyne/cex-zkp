param(
    [string] $Api = "",
    [int]    $DepthLimit = 20,
    [int]    $TradesLimit = 40,
    [switch] $Seed
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$pairs = @("BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT")
$baseUrl = if ($env:BINANCE_API_BASE_URL -and $env:BINANCE_API_BASE_URL.Trim() -ne "") {
    $env:BINANCE_API_BASE_URL.TrimEnd('/')
} else {
    "https://api.binance.com/api/v3"
}
$binanceApiKey = if ($env:BINANCE_API_KEY) { $env:BINANCE_API_KEY.Trim() } else { "" }
$dataDir = "data"

if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir | Out-Null
}

function Invoke-BinanceGet {
    param([string] $Path)
    $headers = @{}
    if ($binanceApiKey -ne "") {
        $headers["X-MBX-APIKEY"] = $binanceApiKey
    }
    return Invoke-RestMethod -Method Get -Uri "$baseUrl/$Path" -Headers $headers -TimeoutSec 15
}

function Seed-OrdersForMarket {
    param(
        [string] $ApiUrl,
        [string] $Pair,
        [object] $Depth
    )

    $base = $Pair -replace "USDT", ""

    foreach ($entry in $Depth.asks) {
        $body = @{
            side = "sell"
            price = $entry[0]
            amount = $entry[1]
            base_asset = $base
            quote_asset = "USDT"
        } | ConvertTo-Json -Compress

        Invoke-RestMethod -Uri "$ApiUrl/api/orders" -Method Post -Headers @{
            "x-user-id" = "1"
            "Content-Type" = "application/json"
        } -Body $body -TimeoutSec 10 | Out-Null
    }

    foreach ($entry in $Depth.bids) {
        $body = @{
            side = "buy"
            price = $entry[0]
            amount = $entry[1]
            base_asset = $base
            quote_asset = "USDT"
        } | ConvertTo-Json -Compress

        Invoke-RestMethod -Uri "$ApiUrl/api/orders" -Method Post -Headers @{
            "x-user-id" = "2"
            "Content-Type" = "application/json"
        } -Body $body -TimeoutSec 10 | Out-Null
    }
}

$fetchedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

Write-Host "Fetching latest market data at $fetchedAt ..."

$depthByPair = @{}
$tradesByPair = @{}
$tickerByPair = @{}

foreach ($pair in $pairs) {
    Write-Host "  -> $pair"
    $depthByPair[$pair] = Invoke-BinanceGet "depth?symbol=$pair&limit=$DepthLimit"
    $tradesByPair[$pair] = Invoke-BinanceGet "aggTrades?symbol=$pair&limit=$TradesLimit"
    $tickerByPair[$pair] = Invoke-BinanceGet "ticker/24hr?symbol=$pair"
}

$snapshotPayload = [ordered]@{
    fetched_at = $fetchedAt
    source = "Binance REST API"
    depth_limit = $DepthLimit
    markets = @()
}

foreach ($pair in $pairs) {
    $snapshotPayload.markets += [ordered]@{
        symbol = $pair
        last_update_id = $depthByPair[$pair].lastUpdateId
        bids = $depthByPair[$pair].bids
        asks = $depthByPair[$pair].asks
    }
}

$tradesPayload = [ordered]@{
    fetched_at = $fetchedAt
    source = "Binance REST API"
    trades_limit = $TradesLimit
    markets = @()
}

foreach ($pair in $pairs) {
    $tradesPayload.markets += [ordered]@{
        symbol = $pair
        trades = $tradesByPair[$pair]
    }
}

$tickerPayload = [ordered]@{
    fetched_at = $fetchedAt
    source = "Binance REST API"
    markets = @()
}

foreach ($pair in $pairs) {
    $t = $tickerByPair[$pair]
    $tickerPayload.markets += [ordered]@{
        symbol = $pair
        last_price = $t.lastPrice
        price_change_percent_24h = $t.priceChangePercent
        quote_volume_24h = $t.quoteVolume
    }
}

$snapshotPath = Join-Path $dataDir "markets_snapshot.json"
$tradesPath = Join-Path $dataDir "markets_trades.json"
$tickerPath = Join-Path $dataDir "markets_ticker_24h.json"

$snapshotPayload | ConvertTo-Json -Depth 10 | Set-Content $snapshotPath -Encoding UTF8
$tradesPayload | ConvertTo-Json -Depth 10 | Set-Content $tradesPath -Encoding UTF8
$tickerPayload | ConvertTo-Json -Depth 10 | Set-Content $tickerPath -Encoding UTF8

Write-Host "Saved: $snapshotPath"
Write-Host "Saved: $tradesPath"
Write-Host "Saved: $tickerPath"

if ($Seed) {
    if ([string]::IsNullOrWhiteSpace($Api)) {
        throw "When using -Seed, you must provide -Api (example: -Api http://localhost:3000)."
    }

    Write-Host "Seeding orderbook data to $Api ..."
    foreach ($pair in $pairs) {
        Seed-OrdersForMarket -ApiUrl $Api -Pair $pair -Depth $depthByPair[$pair]
    }
    Write-Host "Seed completed for BTC/ETH/SOL/BNB against USDT."
}

Write-Host "Done. Data timestamp is current runtime UTC: $fetchedAt"
