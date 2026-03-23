<script lang="ts">
  import { onMount } from "svelte";
  import { router } from "../../stores/routerStore";
  import { logoutAdmin } from "../../stores/adminAuthStore";
  import {
    fetchAdminMetrics,
    fetchTreasuryMetrics,
    fetchAdminAssets,
    fetchAdminUsers,
    suspendUser,
    haltMarket,
    triggerZkpSnapshot,
    type AdminMetrics,
    type TreasuryMetrics,
    type AdminAssetDto,
    type AdminUserDto
  } from "../../lib/api/client";

  // Dashboard state
  let metrics = $state<AdminMetrics | null>(null);
  let treasury = $state<TreasuryMetrics | null>(null);
  
  // Assets state
  let assets = $state<AdminAssetDto[]>([]);
  
  // Users state
  let users = $state<AdminUserDto[]>([]);
  
  // UI Tab state
  let activeTab = $state<"dashboard" | "assets" | "users" | "zkp">("dashboard");

  // Notifications
  let message = $state<string | null>(null);

  async function loadDashboard() {
    try {
      metrics = await fetchAdminMetrics();
      treasury = await fetchTreasuryMetrics();
    } catch (e) {
      console.error(e);
    }
  }

  async function loadAssets() {
    try {
      assets = await fetchAdminAssets();
    } catch (e) {
      console.error(e);
    }
  }

  async function loadUsers() {
    try {
      users = await fetchAdminUsers();
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    loadDashboard();
  });

  $effect(() => {
    if (activeTab === "dashboard") loadDashboard();
    if (activeTab === "assets") loadAssets();
    if (activeTab === "users") loadUsers();
  });

  async function handleSuspend(userId: number) {
    if (!confirm(`Are you sure you want to suspend user ${userId}?`)) return;
    try {
      await suspendUser(userId);
      message = `User ${userId} suspended successfully.`;
      loadUsers();
      setTimeout(() => message = null, 3000);
    } catch (e: any) {
      alert("Error: " + e.message);
    }
  }

  async function handleHaltMarket(symbol: string) {
    if (!confirm(`Are you sure you want to HALT trading for ${symbol}?`)) return;
    try {
      await haltMarket(symbol);
      message = `Market ${symbol} halted successfully.`;
      setTimeout(() => message = null, 3000);
    } catch (e: any) {
      alert("Error: " + e.message);
    }
  }

  async function handleZkpSnapshot() {
    if (!confirm("Are you sure you want to trigger a global balance snapshot for ZKP?")) return;
    try {
      const res = await triggerZkpSnapshot();
      message = `Snapshot triggered successfully.`;
      setTimeout(() => message = null, 3000);
    } catch (e: any) {
      alert("Error: " + e.message);
    }
  }
</script>

<div class="space-y-6 max-w-6xl mx-auto">
  <!-- Header -->
  <div class="px-4 py-3 bg-slate-900 border border-slate-800 rounded-xl flex items-center justify-between">
    <div>
      <h1 class="text-sm font-semibold tracking-widest text-slate-100 uppercase">Admin Dashboard</h1>
      <p class="text-xs text-slate-500 mt-1">Management and Monitoring</p>
    </div>
    
<div class="flex items-center gap-4">
      <div class="flex gap-2">
        <button
          class="px-3 py-1.5 text-xs font-semibold rounded-md border {activeTab === 'dashboard' ? 'bg-sky-500/20 text-sky-400 border-sky-500/50' : 'bg-slate-800 text-slate-400 border-slate-700'}"
          onclick={() => activeTab = 'dashboard'}>Dashboard</button>
        <button
          class="px-3 py-1.5 text-xs font-semibold rounded-md border {activeTab === 'assets' ? 'bg-sky-500/20 text-sky-400 border-sky-500/50' : 'bg-slate-800 text-slate-400 border-slate-700'}"
          onclick={() => activeTab = 'assets'}>Markets & Assets</button>
        <button
          class="px-3 py-1.5 text-xs font-semibold rounded-md border {activeTab === 'users' ? 'bg-sky-500/20 text-sky-400 border-sky-500/50' : 'bg-slate-800 text-slate-400 border-slate-700'}"
          onclick={() => activeTab = 'users'}>Users</button>
        <button
          class="px-3 py-1.5 text-xs font-semibold rounded-md border {activeTab === 'zkp' ? 'bg-sky-500/20 text-sky-400 border-sky-500/50' : 'bg-slate-800 text-slate-400 border-slate-700'}"
          onclick={() => activeTab = 'zkp'}>ZKP Audit</button>
      </div>

      <div class="h-6 w-px bg-slate-700"></div>

      <button
        class="px-3 py-1.5 text-xs font-semibold rounded-md border bg-slate-800 text-slate-300 border-slate-700 hover:border-slate-500 transition-colors"
        onclick={() => { logoutAdmin(); router.navigate("/admin/login"); }}
      >
        Logout
      </button>
    </div>
  </div>

  {#if message}
    <div class="bg-emerald-500/20 border border-emerald-500/50 text-emerald-400 px-4 py-2 rounded-lg text-sm text-center">
      {message}
    </div>
  {/if}

  <!-- Tab Content -->
  {#if activeTab === "dashboard"}
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <div class="terminal-panel p-5">
        <h2 class="text-xs font-medium text-slate-400 mb-4 uppercase tracking-widest border-b border-slate-800 pb-2">Exchange Metrics</h2>
        {#if metrics}
          <div class="space-y-4">
            <div class="flex justify-between items-center bg-slate-900/50 p-3 rounded border border-slate-800/50">
              <span class="text-sm text-slate-400">24h Volume (USDT)</span>
              <span class="text-sm font-bold text-sky-400 mono">${parseFloat(metrics.volume_24h_usdt).toLocaleString()}</span>
            </div>
            <div class="flex justify-between items-center bg-slate-900/50 p-3 rounded border border-slate-800/50">
              <span class="text-sm text-slate-400">Total Users</span>
              <span class="text-sm font-bold text-slate-200 mono">{metrics.total_users}</span>
            </div>
            <div class="flex justify-between items-center bg-slate-900/50 p-3 rounded border border-slate-800/50">
              <span class="text-sm text-slate-400">Active Orders</span>
              <span class="text-sm font-bold text-slate-200 mono">{metrics.active_orders}</span>
            </div>
          </div>
        {:else}
          <p class="text-sm text-slate-600">Loading metrics...</p>
        {/if}
      </div>

      <div class="terminal-panel p-5">
        <h2 class="text-xs font-medium text-slate-400 mb-4 uppercase tracking-widest border-b border-slate-800 pb-2">Treasury & Solvency (USDT)</h2>
        {#if treasury}
          <div class="space-y-4">
            <div class="flex justify-between items-center bg-slate-900/50 p-3 rounded border border-slate-800/50">
              <span class="text-sm text-slate-400">Total Assets (Exchange Wallet)</span>
              <span class="text-sm font-bold text-emerald-400 mono">${parseFloat(treasury.total_exchange_funds).toLocaleString()}</span>
            </div>
            <div class="flex justify-between items-center bg-slate-900/50 p-3 rounded border border-slate-800/50">
              <span class="text-sm text-slate-400">Total Liabilities (User Balances)</span>
              <span class="text-sm font-bold text-rose-400 mono">${parseFloat(treasury.total_user_liabilities).toLocaleString()}</span>
            </div>
            <div class="flex justify-between items-center bg-slate-900/50 p-3 rounded border border-slate-800/50">
              <span class="text-sm text-slate-400">Solvency Ratio</span>
              <span class="text-sm font-bold text-sky-400 mono">{treasury.solvency_ratio}</span>
            </div>
          </div>
        {:else}
          <p class="text-sm text-slate-600">Loading treasury...</p>
        {/if}
      </div>
    </div>
  {/if}

  {#if activeTab === "assets"}
    <div class="terminal-panel p-5">
        <h2 class="text-xs font-medium text-slate-400 mb-4 uppercase tracking-widest border-b border-slate-800 pb-2">Supported Assets & Markets</h2>
        <table class="w-full text-left text-sm mb-6">
          <thead class="text-xs text-slate-500 uppercase bg-slate-900/80">
            <tr>
              <th class="py-2 px-3">Symbol</th>
              <th class="py-2 px-3">Name</th>
              <th class="py-2 px-3">Status</th>
              <th class="py-2 px-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each assets as asset}
            <tr class="border-b border-slate-800">
              <td class="py-2 px-3 font-medium text-slate-200">{asset.symbol}</td>
              <td class="py-2 px-3 text-slate-400">{asset.name}</td>
              <td class="py-2 px-3">
                <span class={asset.is_active ? 'text-emerald-400' : 'text-slate-500'}>
                  {asset.is_active ? 'Active' : 'Inactive'}
                </span>
              </td>
              <td class="py-2 px-3 text-right">
                {#if asset.is_active && asset.symbol === "BTC"}
                  <button class="text-xs bg-rose-500/20 text-rose-400 border border-rose-500/30 px-2 py-1 rounded hover:bg-rose-500/30"
                          onclick={() => handleHaltMarket(`${asset.symbol}_USDT`)}>
                    Halt Market
                  </button>
                {/if}
              </td>
            </tr>
            {/each}
          </tbody>
        </table>
        
        <div class="bg-slate-900 border border-slate-800 rounded-lg p-4">
          <h3 class="text-xs text-slate-400 uppercase tracking-widest mb-3">Add New Asset</h3>
          <div class="flex gap-3">
            <input type="text" placeholder="Symbol (e.g. ADA)" class="bg-slate-950 border border-slate-700 rounded px-3 py-1.5 text-sm w-32 focus:outline-none focus:border-sky-500 text-white" />
            <input type="text" placeholder="Name" class="bg-slate-950 border border-slate-700 rounded px-3 py-1.5 text-sm w-48 focus:outline-none focus:border-sky-500 text-white" />
            <button class="bg-sky-600 hover:bg-sky-500 text-white px-4 py-1.5 rounded text-sm font-medium transition-colors">Add</button>
          </div>
        </div>
    </div>
  {/if}

  {#if activeTab === "users"}
    <div class="terminal-panel p-5">
      <h2 class="text-xs font-medium text-slate-400 mb-4 uppercase tracking-widest border-b border-slate-800 pb-2">User Management</h2>
      <table class="w-full text-left text-sm">
        <thead class="text-xs text-slate-500 uppercase bg-slate-900/80">
          <tr>
            <th class="py-2 px-3">User ID</th>
            <th class="py-2 px-3">Username</th>
            <th class="py-2 px-3">Status</th>
            <th class="py-2 px-3 text-right">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each users as user}
          <tr class="border-b border-slate-800">
            <td class="py-2 px-3 font-medium text-slate-400">{user.user_id}</td>
            <td class="py-2 px-3 text-slate-200">{user.username}</td>
            <td class="py-2 px-3">
              <span class={user.is_suspended ? 'text-rose-400' : 'text-emerald-400'}>
                {user.is_suspended ? 'Suspended' : 'Active'}
              </span>
            </td>
            <td class="py-2 px-3 text-right">
              {#if !user.is_suspended}
                <button class="text-xs bg-rose-500/20 text-rose-400 border border-rose-500/30 px-2 py-1 rounded hover:bg-rose-500/30"
                        onclick={() => handleSuspend(user.user_id)}>
                  Suspend
                </button>
              {/if}
            </td>
          </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if activeTab === "zkp"}
    <div class="terminal-panel p-5 text-center">
      <h2 class="text-xs font-medium text-slate-400 mb-4 uppercase tracking-widest border-b border-slate-800 pb-2">ZKP Audit Operations</h2>
      
      <div class="py-8">
        <p class="text-sm text-slate-300 mb-6 max-w-lg mx-auto leading-relaxed">
          Triggering a Snapshot will collect the balances of all users alongside the main exchange wallet to construct a new Merkle Sum Tree. This is required before users can verify their Proof of Solvency.
        </p>
        <button class="bg-indigo-600 hover:bg-indigo-500 text-white px-6 py-2.5 rounded-lg text-sm font-semibold tracking-wide transition-all shadow-[0_0_15px_rgba(79,70,229,0.3)] hover:shadow-[0_0_20px_rgba(79,70,229,0.5)]"
                onclick={handleZkpSnapshot}>
          CRON: Execute Snapshot & Hash
        </button>
      </div>

      <div class="mt-4 text-left border-t border-slate-800 pt-4">
        <h3 class="text-xs text-slate-500 uppercase tracking-widest mb-3">Recent ZKP Snapshots</h3>
        <div class="bg-slate-900 border border-slate-800 rounded py-2 px-3 text-sm text-slate-400 mono">
          <p>ID: snap_20260323_01 • Hash: 0x8a92...eb14 • Users: 4</p>
        </div>
      </div>
    </div>
  {/if}
</div>
