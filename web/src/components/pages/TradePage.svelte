<script lang="ts">
  import OrderBookPanel from "../orderbook/OrderBookPanel.svelte";
  import TradeFormPanel from "../trade/TradeFormPanel.svelte";
  import SimulatorPanel from "../simulator/SimulatorPanel.svelte";
  import OpenOrdersPanel from "../trade/OpenOrdersPanel.svelte";
  import RecentTradesPanel from "../trade/RecentTradesPanel.svelte";
</script>

<div class="grid gap-4 lg:grid-cols-[1.45fr_1fr] lg:items-start md:gap-6">
  <div class="space-y-4 md:space-y-6">
    <OrderBookPanel />
    <RecentTradesPanel />
  </div>

  <div class="space-y-4 md:space-y-6">
    <TradeFormPanel />
    <OpenOrdersPanel />
    <SimulatorPanel />
  </div>
</div>
