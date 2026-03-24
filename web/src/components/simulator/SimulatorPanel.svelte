<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { SUPPORTED_MARKET_ASSETS } from "../../lib/marketMeta";
  import {
    fetchLiveTickers,
    fetchSimulatorStatus,
    resetSimulator,
    setSimulatorProfile,
    startSimulator,
    stopSimulator,
    type SimProfileKey,
    type SimulatorStatus,
  } from "../../lib/api/client";

  type PairStats = {
    pair: string;
    price: number | null;
    changePct: number;
    orders: number;
    fills: number;
  };

  const INITIAL_PAIR_STATS: Record<string, PairStats> = {
    BTC_USDT: { pair: "BTC_USDT", price: null, changePct: 0, orders: 0, fills: 0 },
    ETH_USDT: { pair: "ETH_USDT", price: null, changePct: 0, orders: 0, fills: 0 },
    SOL_USDT: { pair: "SOL_USDT", price: null, changePct: 0, orders: 0, fills: 0 },
    BNB_USDT: { pair: "BNB_USDT", price: null, changePct: 0, orders: 0, fills: 0 },
  };

  const PAIRS = SUPPORTED_MARKET_ASSETS.map((m) => ({
    pair: m.pair,
    symbol: m.symbol,
  }));

  const SIM_PROFILES: Record<
    SimProfileKey,
    { intervalMs: number; ordersPerPairPerTick: number; aggressionRate: number; amountMax: number }
  > = {
    normal: { intervalMs: 550, ordersPerPairPerTick: 3, aggressionRate: 0.45, amountMax: 0.08 },
    fast: { intervalMs: 250, ordersPerPairPerTick: 8, aggressionRate: 0.58, amountMax: 0.18 },
    turbo: { intervalMs: 120, ordersPerPairPerTick: 16, aggressionRate: 0.7, amountMax: 0.35 },
    hyper: { intervalMs: 70, ordersPerPairPerTick: 28, aggressionRate: 0.8, amountMax: 0.5 },
  };

  let isRunning = $state(true);
  let profileKey = $state<SimProfileKey>("turbo");
  let totalOrders = $state(0);
  let totalFills = $state(0);
  let ticks = $state(0);

  let tickerRef: ReturnType<typeof setInterval> | null = null;
  let statusRef: ReturnType<typeof setInterval> | null = null;

  let pairStats = $state<Record<string, PairStats>>({ ...INITIAL_PAIR_STATS });

  let estOrdersPerSec = $derived(
    Math.round((1000 / SIM_PROFILES[profileKey].intervalMs) * (SIM_PROFILES[profileKey].ordersPerPairPerTick * PAIRS.length)),
  );

  function rand(min: number, max: number) {
    return min + Math.random() * (max - min);
  }

  function formatPrice(value: number | null): string {
    if (value === null || !Number.isFinite(value)) {
      return "--";
    }
    return value.toFixed(2);
  }

  function applyStatus(status: SimulatorStatus) {
    isRunning = status.running;
    profileKey = status.profile;
    totalOrders = status.total_orders;
    totalFills = status.total_fills;
    ticks = status.ticks;

    const nextStats = { ...pairStats };
    for (const [pair, stats] of Object.entries(status.pair_stats)) {
      const current = nextStats[pair] ?? INITIAL_PAIR_STATS[pair] ?? { pair, price: null, changePct: 0, orders: 0, fills: 0 };
      nextStats[pair] = {
        ...current,
        orders: stats.orders,
        fills: stats.fills,
      };
    }
    pairStats = nextStats;
  }

  async function refreshStatus() {
    try {
      const status = await fetchSimulatorStatus();
      applyStatus(status);
    } catch {
      // keep last known status if API is temporarily unavailable
    }
  }

  async function refreshTicker() {
    try {
      const data = await fetchLiveTickers(PAIRS.map((m) => `${m.symbol}USDT`));
      for (const item of data) {
        const pair = item.symbol.replace("USDT", "") + "_USDT";
        const p = parseFloat(item.last_price);
        const ch = parseFloat(item.price_change_percent_24h);
        if (!Number.isFinite(p) || !pairStats[pair]) continue;

        pairStats = {
          ...pairStats,
          [pair]: {
            ...pairStats[pair],
            price: p,
            changePct: Number.isFinite(ch) ? ch : pairStats[pair].changePct,
          },
        };
      }
    } catch {
      // Keep previous anchors if network hiccups happen.
    }
  }

  async function start() {
    await startSimulator(profileKey);
    await refreshStatus();
  }

  async function stop() {
    await stopSimulator();
    await refreshStatus();
  }

  async function toggle() {
    if (isRunning) {
      await stop();
      return;
    }
    await start();
  }

  async function setProfile(next: SimProfileKey) {
    profileKey = next;
    await setSimulatorProfile(next);
    await refreshStatus();
  }

  async function reset() {
    await resetSimulator();
    await refreshStatus();
  }

  onMount(() => {
    void start();
    void refreshStatus();
    void refreshTicker();
    tickerRef = setInterval(() => {
      void refreshTicker();
    }, 2500);
    statusRef = setInterval(() => {
      void refreshStatus();
    }, 1000);
  });

  onDestroy(() => {
    if (tickerRef) {
      clearInterval(tickerRef);
      tickerRef = null;
    }
    if (statusRef) {
      clearInterval(statusRef);
      statusRef = null;
    }
  });
</script>

<section class="terminal-panel-strong p-4 sm:p-5 relative overflow-hidden">
  <!-- Header -->
  <div class="mb-4 flex items-center justify-between">
    <div class="flex items-center gap-2">
      <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Bot Simulator</h2>
      {#if isRunning}
        <span class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
        </span>
      {/if}
    </div>
    <span class="mono text-[10px] text-slate-500 bg-slate-800/50 px-2 py-0.5 rounded">4 markets / realtime anchor</span>
  </div>

  <!-- Realtime anchors by market -->
  <div class="mb-4 grid grid-cols-2 gap-2 text-center">
    {#each PAIRS as m}
      {@const stats = pairStats[m.pair] ?? INITIAL_PAIR_STATS[m.pair]!}
      <div class="rounded-lg border border-slate-800 bg-slate-900/60 py-2 px-2">
        <p class="text-[9px] uppercase tracking-widest text-slate-500">{m.pair.replace('_', '/')}</p>
        <p class="mono text-sm font-semibold text-fuchsia-300">${formatPrice(stats.price)}</p>
        <p class="mono text-[10px] {stats.changePct >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
          {stats.changePct >= 0 ? '+' : ''}{stats.changePct.toFixed(2)}%
        </p>
      </div>
    {/each}
  </div>

  <!-- Stats -->
  <div class="mb-4 grid grid-cols-3 gap-2 text-center">
    <div class="rounded-lg border border-slate-800 bg-slate-900/50 py-2">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Orders</p>
      <p class="text-lg font-bold mono text-slate-200">{totalOrders}</p>
    </div>
    <div class="rounded-lg border border-slate-800 bg-slate-900/50 py-2">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Fills</p>
      <p class="text-lg font-bold mono text-emerald-400">{totalFills}</p>
    </div>
    <div class="rounded-lg border border-slate-800 bg-slate-900/50 py-2">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Est O/s</p>
      <p class="text-lg font-bold mono text-cyan-300">{estOrdersPerSec}</p>
    </div>
  </div>

  <div class="mb-4 grid grid-cols-2 gap-2 text-center">
    {#each PAIRS as m}
      {@const stats = pairStats[m.pair] ?? INITIAL_PAIR_STATS[m.pair]!}
      <div class="rounded-lg border border-slate-800 bg-slate-900/50 py-2 px-2">
        <p class="text-[9px] uppercase tracking-widest text-slate-500">{m.symbol} filled/orders</p>
        <p class="mono text-xs text-slate-200">{stats.fills} / {stats.orders}</p>
      </div>
    {/each}
  </div>

  <!-- Throughput profile selector -->
  <div class="mb-4">
    <p class="mb-1.5 text-[10px] uppercase tracking-widest text-slate-500">Throughput profile</p>
    <div class="grid grid-cols-4 gap-1.5">
      {#each (["normal", "fast", "turbo", "hyper"] as const) as s}
        <button
          type="button"
          onclick={() => setProfile(s)}
          class="rounded-md border py-1 text-[11px] font-medium uppercase tracking-wide transition
            {profileKey === s
              ? 'border-sky-500/50 bg-sky-500/20 text-sky-300'
              : 'border-slate-700/60 bg-slate-900/60 text-slate-500 hover:border-slate-600 hover:text-slate-300'}"
        >
          {s}
        </button>
      {/each}
    </div>
  </div>

  <!-- Start / Stop + Reset -->
  <div class="flex gap-2">
    <button
      type="button"
      onclick={toggle}
      class="h-9 flex-1 rounded-lg border text-sm font-semibold uppercase tracking-wider transition
        {isRunning
          ? 'border-rose-500/30 bg-rose-500/20 text-rose-300 hover:bg-rose-500/30'
          : 'border-emerald-500/30 bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30'}"
    >
      {isRunning ? "⏹ Stop" : "▶ Start"}
    </button>
    <button
      type="button"
      onclick={reset}
      class="h-9 rounded-lg border border-slate-700/60 bg-slate-900/60 px-3 text-xs uppercase
             tracking-wide text-slate-500 transition hover:border-slate-600 hover:text-slate-300"
    >
      Reset
    </button>
  </div>

  <p class="mt-3 text-[10px] text-slate-500 mono">
    Tick: {SIM_PROFILES[profileKey].intervalMs}ms • Orders/tick: {SIM_PROFILES[profileKey].ordersPerPairPerTick * PAIRS.length} • Ticks: {ticks}
  </p>
</section>

