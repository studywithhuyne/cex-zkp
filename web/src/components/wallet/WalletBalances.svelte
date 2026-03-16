<script lang="ts">
  import { selectedUserId, balanceVersion } from "../../stores/appStore";
  import { fetchBalances } from "../../lib/api/client";
  import type { BalanceDto } from "../../lib/api/client";

  let balances = $state<BalanceDto[]>([]);
  let isLoading = $state(false);

  async function load() {
    isLoading = true;
    try {
      balances = await fetchBalances($selectedUserId);
    } catch {
      balances = [];
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    void $selectedUserId;
    void $balanceVersion;
    load();
  });
</script>

<section class="terminal-panel-strong p-4 sm:p-5">
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Portfolio Balances</h2>
    <button
      onclick={load}
      class="rounded px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider
        bg-sky-500/10 text-sky-400 border border-sky-500/20 hover:bg-sky-500/20 transition"
    >
      Refresh
    </button>
  </div>

  {#if isLoading && balances.length === 0}
    <div class="flex items-center justify-center py-8 text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
  {:else if balances.length === 0}
    <div class="flex items-center justify-center py-8 text-[10px] text-slate-600 uppercase tracking-widest">No balances found</div>
  {:else}
    <div class="grid gap-3 sm:grid-cols-2">
      {#each balances as bal}
        <div class="rounded-lg border border-slate-800/50 bg-slate-900/40 p-4">
          <div class="flex items-center justify-between mb-2">
            <span class="text-sky-300 font-bold text-sm tracking-wider">{bal.asset}</span>
          </div>
          <div class="space-y-1">
            <div class="flex justify-between">
              <span class="text-[11px] text-slate-500 uppercase tracking-wider">Available</span>
              <span class="mono text-sm text-slate-200">{parseFloat(bal.available).toLocaleString(undefined, { maximumFractionDigits: 8 })}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-[11px] text-slate-500 uppercase tracking-wider">Locked</span>
              <span class="mono text-sm text-slate-400">{parseFloat(bal.locked).toLocaleString(undefined, { maximumFractionDigits: 8 })}</span>
            </div>
            <div class="flex justify-between border-t border-slate-800/40 pt-1">
              <span class="text-[11px] text-slate-500 uppercase tracking-wider">Total</span>
              <span class="mono text-sm font-medium text-slate-100">
                {(parseFloat(bal.available) + parseFloat(bal.locked)).toLocaleString(undefined, { maximumFractionDigits: 8 })}
              </span>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>
