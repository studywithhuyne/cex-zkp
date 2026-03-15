<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { orderBook } from "./stores/orderBookStore.js";
  import TopBar from "./components/layout/TopBar.svelte";
  import UserHeader from "./components/user/UserHeader.svelte";
  import OrderBookPanel from "./components/orderbook/OrderBookPanel.svelte";
  import TradeFormPanel from "./components/trade/TradeFormPanel.svelte";
  import ZkpVerifierPanel from "./components/zkp/ZkpVerifierPanel.svelte";

  onMount(() => {
    orderBook.connect();
  });

  onDestroy(() => {
    orderBook.disconnect();
  });
</script>

<main class="terminal-shell">
  <TopBar />

  <div class="mt-4 md:mt-6">
    <UserHeader />
  </div>

  <div class="mt-4 grid gap-4 lg:grid-cols-[1.45fr_1fr] lg:items-start md:mt-6 md:gap-6">
    <OrderBookPanel />

    <div class="space-y-4 md:space-y-6">
      <TradeFormPanel />
      <ZkpVerifierPanel />
    </div>
  </div>

  <footer class="mt-5 flex flex-wrap items-center gap-2 text-xs text-slate-400 md:mt-7">
    <span class="mono rounded-full border border-slate-700/70 bg-slate-900/70 px-2 py-1">Nginx SPA</span>
    <span class="mono rounded-full border border-slate-700/70 bg-slate-900/70 px-2 py-1">/api proxied</span>
    <span class="mono rounded-full border border-slate-700/70 bg-slate-900/70 px-2 py-1">/ws ready</span>
  </footer>
</main>
