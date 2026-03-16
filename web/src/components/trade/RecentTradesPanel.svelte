<script lang="ts">
  import { onMount } from "svelte";
  import { orderBook } from "../../stores/orderBookStore";
  import { fetchRecentTrades } from "../../lib/api/client";
  import type { RecentTrade } from "../../lib/api/client";

  let historicalTrades = $state<RecentTrade[]>([]);

  onMount(async () => {
    try {
      historicalTrades = await fetchRecentTrades();
    } catch {
      historicalTrades = [];
    }
  });

  // Merge: WS trades first (realtime), then historical
  let displayTrades = $derived((() => {
    const wsTrades = $orderBook.trades.map((t: { price: number; amount: number }) => ({
      price: String(t.price),
      amount: String(t.amount),
      base_asset: "BTC",
      quote_asset: "USDT",
      executed_at: new Date().toISOString(),
    }));
    const combined = [...wsTrades, ...historicalTrades];
    return combined.slice(0, 50);
  })());

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleTimeString();
    } catch {
      return "--:--:--";
    }
  }
</script>

<section class="terminal-panel p-4 sm:p-5">
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Recent Trades</h2>
    <span class="mono text-[10px] text-slate-500">{displayTrades.length} trade{displayTrades.length !== 1 ? "s" : ""}</span>
  </div>

  {#if displayTrades.length === 0}
    <div class="flex items-center justify-center py-6 text-[10px] text-slate-600 uppercase tracking-widest">No trades yet</div>
  {:else}
    <div class="overflow-y-auto max-h-64 hide-scrollbar">
      <table class="w-full text-xs">
        <thead class="sticky top-0 bg-slate-950/90 backdrop-blur">
          <tr class="border-b border-slate-800/60 text-[10px] text-slate-500 uppercase tracking-wider">
            <th class="py-1.5 text-left font-medium">Time</th>
            <th class="py-1.5 text-right font-medium">Price</th>
            <th class="py-1.5 text-right font-medium">Amount</th>
            <th class="py-1.5 text-right font-medium">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each displayTrades as trade}
            <tr class="border-b border-slate-800/20">
              <td class="py-1 mono text-[10px] text-slate-500">{formatTime(trade.executed_at)}</td>
              <td class="py-1 text-right mono text-slate-200">{parseFloat(trade.price).toLocaleString()}</td>
              <td class="py-1 text-right mono text-slate-300">{trade.amount}</td>
              <td class="py-1 text-right mono text-slate-400">
                {(parseFloat(trade.price) * parseFloat(trade.amount)).toLocaleString(undefined, { maximumFractionDigits: 2 })}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>

<style>
  .hide-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .hide-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>

