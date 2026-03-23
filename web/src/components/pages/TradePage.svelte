<script lang="ts">
  import OrderBookPanel from "../orderbook/OrderBookPanel.svelte";
  import TradeFormPanel from "../trade/TradeFormPanel.svelte";
  import OpenOrdersPanel from "../trade/OpenOrdersPanel.svelte";
  import RecentTradesPanel from "../trade/RecentTradesPanel.svelte";
  import ChartPanel from "../trading/ChartPanel.svelte";
  import MarketTicker from "../trade/MarketTicker.svelte";
</script>

<div class="flex flex-col">
  <MarketTicker />

  <div class="grid gap-4 lg:grid-cols-[1.45fr_1fr] lg:items-start md:gap-6">
    <div class="flex flex-col space-y-4 md:space-y-6">
      <ChartPanel />
      <OrderBookPanel />
      <RecentTradesPanel />
    </div>

    <div class="space-y-4 md:space-y-6">
      <TradeFormPanel />
      <OpenOrdersPanel />
    </div>
  </div>
</div>

