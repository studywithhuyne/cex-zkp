<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { orderBook } from "./stores/orderBookStore.js";
  import { router } from "./stores/routerStore.js";
  import Navbar from "./components/layout/Navbar.svelte";
  import TradePage from "./components/pages/TradePage.svelte";
  import WalletPage from "./components/pages/WalletPage.svelte";
  import ZkVerifyPage from "./components/pages/ZkVerifyPage.svelte";

  onMount(() => {
    orderBook.connect();
  });

  onDestroy(() => {
    orderBook.disconnect();
  });
</script>

<div class="terminal-shell">
  <Navbar />

  <main class="mt-4 md:mt-6">
    {#if $router === "/trade"}
      <TradePage />
    {:else if $router === "/wallet"}
      <WalletPage />
    {:else if $router === "/zk-verify"}
      <ZkVerifyPage />
    {/if}
  </main>

  <footer class="mt-5 flex flex-wrap items-center gap-2 text-xs text-slate-400 md:mt-7">
    <span class="mono rounded-full border border-slate-700/70 bg-slate-900/70 px-2 py-1">Nginx SPA</span>
    <span class="mono rounded-full border border-slate-700/70 bg-slate-900/70 px-2 py-1">/api proxied</span>
    <span class="mono rounded-full border border-slate-700/70 bg-slate-900/70 px-2 py-1">/ws ready</span>
  </footer>
</div>
