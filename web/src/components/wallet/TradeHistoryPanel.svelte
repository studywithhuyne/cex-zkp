<script lang="ts">
  import { authState } from '../../stores/authStore';
  import { fetchUserTrades } from "../../lib/api/client";
  import type { UserTrade } from "../../lib/api/client";

  let trades = $state<UserTrade[]>([]);
  let isLoading = $state(false);
  let timeFilter = $state<"24h" | "7d" | "30d" | "all" | "custom">("7d");
  let fromTime = $state("");
  let toTime = $state("");
  let filteredTrades = $state<UserTrade[]>([]);

  async function load() {
    isLoading = true;
    try {
      trades = await fetchUserTrades(($authState.userId!));
    } catch {
      trades = [];
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    void ($authState.userId!);
    load();
  });

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return "--";
    }
  }

  function isTradeInRange(iso: string): boolean {
    const ts = new Date(iso).getTime();
    if (!Number.isFinite(ts)) return false;

    const now = Date.now();
    if (timeFilter === "24h") return ts >= now - 24 * 60 * 60 * 1000;
    if (timeFilter === "7d") return ts >= now - 7 * 24 * 60 * 60 * 1000;
    if (timeFilter === "30d") return ts >= now - 30 * 24 * 60 * 60 * 1000;
    if (timeFilter === "all") return true;

    const fromTs = fromTime ? new Date(fromTime).getTime() : Number.NEGATIVE_INFINITY;
    const toTs = toTime ? new Date(toTime).getTime() : Number.POSITIVE_INFINITY;
    return ts >= fromTs && ts <= toTs;
  }

  $effect(() => {
    void trades;
    void timeFilter;
    void fromTime;
    void toTime;

    filteredTrades = trades.filter((trade) => isTradeInRange(trade.executed_at));
  });
</script>

<section class="terminal-panel-strong p-4 sm:p-5">
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Trade History</h2>
    <span class="mono text-[10px] text-slate-500">{filteredTrades.length} trade{filteredTrades.length !== 1 ? "s" : ""}</span>
  </div>

  <div class="mb-4 grid grid-cols-1 gap-2 sm:grid-cols-3 lg:grid-cols-5">
    <label class="block lg:col-span-2">
      <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Time Range</span>
      <select
        bind:value={timeFilter}
        class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
      >
        <option value="24h">Last 24 hours</option>
        <option value="7d">Last 7 days</option>
        <option value="30d">Last 30 days</option>
        <option value="all">All</option>
        <option value="custom">Custom</option>
      </select>
    </label>

    {#if timeFilter === "custom"}
      <label class="block">
        <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">From</span>
        <input
          type="datetime-local"
          bind:value={fromTime}
          class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
        />
      </label>

      <label class="block">
        <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">To</span>
        <input
          type="datetime-local"
          bind:value={toTime}
          class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
        />
      </label>
    {/if}
  </div>

  {#if isLoading && filteredTrades.length === 0}
    <div class="flex items-center justify-center py-8 text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
  {:else if filteredTrades.length === 0}
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
          {#each filteredTrades as trade}
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

