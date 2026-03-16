<script lang="ts">
  import { selectedUserId } from "../../stores/appStore";
  import { fetchUserTrades } from "../../lib/api/client";
  import type { UserTrade } from "../../lib/api/client";

  let trades = $state<UserTrade[]>([]);
  let isLoading = $state(false);

  async function load() {
    isLoading = true;
    try {
      trades = await fetchUserTrades($selectedUserId);
    } catch {
      trades = [];
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    void $selectedUserId;
    load();
  });

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return "--";
    }
  }
</script>

<section class="terminal-panel-strong p-4 sm:p-5">
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Trade History</h2>
    <span class="mono text-[10px] text-slate-500">{trades.length} trade{trades.length !== 1 ? "s" : ""}</span>
  </div>

  {#if isLoading && trades.length === 0}
    <div class="flex items-center justify-center py-8 text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
  {:else if trades.length === 0}
    <div class="flex items-center justify-center py-8 text-[10px] text-slate-600 uppercase tracking-widest">No trade history</div>
  {:else}
    <div class="overflow-x-auto max-h-96 overflow-y-auto hide-scrollbar">
      <table class="w-full text-xs">
        <thead class="sticky top-0 bg-slate-950/90 backdrop-blur">
          <tr class="border-b border-slate-800/60 text-[10px] text-slate-500 uppercase tracking-wider">
            <th class="py-1.5 text-left font-medium">Time</th>
            <th class="py-1.5 text-left font-medium">Role</th>
            <th class="py-1.5 text-right font-medium">Price</th>
            <th class="py-1.5 text-right font-medium">Amount</th>
            <th class="py-1.5 text-right font-medium">Total</th>
            <th class="py-1.5 text-right font-medium">Pair</th>
          </tr>
        </thead>
        <tbody>
          {#each trades as trade}
            <tr class="border-b border-slate-800/20 hover:bg-slate-800/20 transition">
              <td class="py-1.5 mono text-[10px] text-slate-500">{formatTime(trade.executed_at)}</td>
              <td class="py-1.5 font-medium uppercase {trade.side === 'taker' ? 'text-sky-400' : 'text-fuchsia-400'}">
                {trade.side}
              </td>
              <td class="py-1.5 text-right mono text-slate-200">{parseFloat(trade.price).toLocaleString()}</td>
              <td class="py-1.5 text-right mono text-slate-300">{trade.amount}</td>
              <td class="py-1.5 text-right mono text-slate-400">
                {(parseFloat(trade.price) * parseFloat(trade.amount)).toLocaleString(undefined, { maximumFractionDigits: 2 })}
              </td>
              <td class="py-1.5 text-right mono text-slate-500 text-[10px]">{trade.base_asset}/{trade.quote_asset}</td>
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
