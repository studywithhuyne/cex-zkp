<script lang="ts">
  import { onMount } from "svelte";
  import { selectedUserId } from "../../stores/appStore";

  const MOCK_USERS = [
    { id: 1, name: "Alice" },
    { id: 2, name: "Bob" },
    { id: 3, name: "Charlie" },
    { id: 4, name: "Dave" }
  ];

  type Balance = {
    asset_symbol: string;
    available: string;
    locked: string;
  };

  let balances = $state<Balance[]>([]);
  let isLoading = $state(false);
  let errorMsg = $state("");

  async function fetchBalances() {
    isLoading = true;
    errorMsg = "";
    try {
      const res = await fetch("/api/balances", {
        headers: { "x-user-id": $selectedUserId.toString() }
      });
      if (!res.ok) throw new Error("Failed to fetch balances");
      balances = await res.json();
    } catch (err: any) {
      errorMsg = err.message;
      balances = [];
    } finally {
      isLoading = false;
    }
  }

  // Refetch balances when user changes
  $effect(() => {
    let _ = $selectedUserId; // dependency tracking
    fetchBalances();
  });
</script>

<section class="terminal-panel-strong p-4 sm:p-5 h-full flex flex-col relative overflow-hidden">
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">User Profile</h2>
    <span class="mono rounded bg-slate-800/80 px-2 py-0.5 text-[10px] text-slate-400">Header: x-user-id</span>
  </div>

  <div class="flex-1 flex flex-col space-y-4">
    <!-- User Spoofing Dropdown -->
    <div class="space-y-1">
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="user-id">Active Mock Trader</label>
      <select
        id="user-id"
        bind:value={$selectedUserId}
        class="mt-2 block w-full rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none ring-0 transition focus:border-sky-500/50 cursor-pointer"
      >
        {#each MOCK_USERS as user}
          <option value={user.id}>{user.name} (ID: {user.id})</option>
        {/each}
      </select>
    </div>

    <!-- Balances Display -->
    <div class="space-y-1 mt-2 flex-1 flex flex-col min-h-25">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase mb-1">Assets (DB Snapshot)</label>
      
      <div class="bg-slate-900/50 border border-slate-800 rounded-lg p-2 flex-1 overflow-y-auto hide-scrollbar">
        {#if isLoading}
          <div class="flex h-full items-center justify-center text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
        {:else if errorMsg}
          <div class="flex h-full items-center justify-center text-[10px] text-rose-500/80 text-center px-2">{errorMsg}</div>
        {:else if balances.length === 0}
          <div class="flex h-full items-center justify-center text-[10px] text-slate-600 uppercase tracking-widest">No Balances</div>
        {:else}
          <ul class="space-y-2">
            {#each balances as bal}
              <li class="flex justify-between items-center bg-slate-800/20 px-2 py-1.5 rounded border border-slate-800/50">
                <span class="text-sky-300 font-bold text-xs tracking-wider">{bal.asset_symbol}</span>
                <div class="text-right">
                  <div class="text-slate-200 text-xs mono">{parseFloat(bal.available).toFixed(2)} <span class="text-[9px] text-slate-500">AVAIL</span></div>
                  {#if parseFloat(bal.locked) > 0}
                    <div class="text-rose-300 text-[10px] mono">{parseFloat(bal.locked).toFixed(2)} <span class="text-[8px] opacity-70">LOCKED</span></div>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  </div>
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
