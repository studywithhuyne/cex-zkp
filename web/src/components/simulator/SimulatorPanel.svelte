<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { SUPPORTED_MARKET_ASSETS } from "../../lib/marketMeta";
  import { fetchLiveTickers } from "../../lib/api/client";

  type SimProfileKey = "normal" | "fast" | "turbo" | "hyper";
  type PairStats = {
    pair: string;
    price: number;
    changePct: number;
    orders: number;
    fills: number;
  };

  const INITIAL_PAIR_STATS: Record<string, PairStats> = {
    BTC_USDT: { pair: "BTC_USDT", price: 65000, changePct: 0, orders: 0, fills: 0 },
    ETH_USDT: { pair: "ETH_USDT", price: 3000, changePct: 0, orders: 0, fills: 0 },
    SOL_USDT: { pair: "SOL_USDT", price: 100, changePct: 0, orders: 0, fills: 0 },
    BNB_USDT: { pair: "BNB_USDT", price: 600, changePct: 0, orders: 0, fills: 0 },
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

  let isRunning = $state(false);
  let profileKey = $state<SimProfileKey>("turbo");
  let totalOrders = $state(0);
  let totalFills = $state(0);
  let ticks = $state(0);
  let inFlight = $state(false);

  let loopRef: ReturnType<typeof setInterval> | null = null;
  let tickerRef: ReturnType<typeof setInterval> | null = null;

  let pairStats = $state<Record<string, PairStats>>({ ...INITIAL_PAIR_STATS });

  let estOrdersPerSec = $derived(
    Math.round((1000 / SIM_PROFILES[profileKey].intervalMs) * (SIM_PROFILES[profileKey].ordersPerPairPerTick * PAIRS.length)),
  );

  function rand(min: number, max: number) {
    return min + Math.random() * (max - min);
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

  function generatePrice(anchor: number, side: "buy" | "sell", aggressive: boolean): number {
    const spread = anchor * 0.0025;
    if (side === "buy") {
      return aggressive ? anchor + rand(0, spread * 0.6) : anchor - rand(spread, spread * 5);
    }
    return aggressive ? anchor - rand(0, spread * 0.6) : anchor + rand(spread, spread * 5);
  }

  async function placeSimOrder(pair: string, anchorPrice: number) {
    const profile = SIM_PROFILES[profileKey];
    const [baseAsset, quoteAsset] = pair.split("_");
    const side = Math.random() < 0.5 ? "buy" : "sell";
    const aggressive = Math.random() < profile.aggressionRate;
    const price = Math.max(0.01, generatePrice(anchorPrice, side, aggressive));
    const amount = rand(0.001, profile.amountMax).toFixed(4);
    const userId = Math.floor(Math.random() * 4) + 1;

    const resp = await fetch("/api/orders", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-user-id": String(userId),
      },
      body: JSON.stringify({
        side,
        price: price.toFixed(2),
        amount,
        base_asset: baseAsset,
        quote_asset: quoteAsset,
      }),
    });

    if (!resp.ok) {
      return { ok: false, fills: 0 };
    }

    const data = await resp.json();
    return { ok: true, fills: Number(data.trades_count ?? 0) };
  }

  async function tick() {
    if (inFlight) return;
    inFlight = true;

    const profile = SIM_PROFILES[profileKey];
    const jobs: Promise<{ pair: string; ok: boolean; fills: number }>[] = [];

    for (const m of PAIRS) {
      const anchor = pairStats[m.pair]?.price ?? 100;
      for (let i = 0; i < profile.ordersPerPairPerTick; i++) {
        jobs.push(
          placeSimOrder(m.pair, anchor)
            .then((r) => ({ pair: m.pair, ok: r.ok, fills: r.fills }))
            .catch(() => ({ pair: m.pair, ok: false, fills: 0 })),
        );
      }
    }

    const settled = await Promise.all(jobs);
    ticks++;

    let ordersDelta = 0;
    let fillsDelta = 0;
    const localPairDelta: Record<string, { orders: number; fills: number }> = {};

    for (const r of settled) {
      const entry = localPairDelta[r.pair] ?? { orders: 0, fills: 0 };
      if (r.ok) {
        ordersDelta += 1;
        fillsDelta += r.fills;
        entry.orders += 1;
        entry.fills += r.fills;
      }
      localPairDelta[r.pair] = entry;
    }

    totalOrders += ordersDelta;
    totalFills += fillsDelta;

    const nextStats = { ...pairStats };
    for (const pair of Object.keys(localPairDelta)) {
      const current = nextStats[pair];
      if (!current) continue;
      const delta = localPairDelta[pair]!;
      nextStats[pair] = {
        ...current,
        orders: current.orders + delta.orders,
        fills: current.fills + delta.fills,
      };
    }
    pairStats = nextStats;

    inFlight = false;
  }

  function start() {
    if (isRunning) return;
    isRunning = true;
    loopRef = setInterval(() => {
      void tick();
    }, SIM_PROFILES[profileKey].intervalMs);
  }

  function stop() {
    if (!isRunning) return;
    isRunning = false;
    if (loopRef) {
      clearInterval(loopRef);
      loopRef = null;
    }
  }

  function toggle() {
    isRunning ? stop() : start();
  }

  function setProfile(next: SimProfileKey) {
    profileKey = next;
    if (isRunning) {
      stop();
      start();
    }
  }

  function reset() {
    stop();
    totalOrders = 0;
    totalFills = 0;
    ticks = 0;
    pairStats = {
      BTC_USDT: { ...pairStats.BTC_USDT!, orders: 0, fills: 0 },
      ETH_USDT: { ...pairStats.ETH_USDT!, orders: 0, fills: 0 },
      SOL_USDT: { ...pairStats.SOL_USDT!, orders: 0, fills: 0 },
      BNB_USDT: { ...pairStats.BNB_USDT!, orders: 0, fills: 0 },
    };
  }

  onMount(() => {
    void refreshTicker();
    tickerRef = setInterval(() => {
      void refreshTicker();
    }, 2500);
  });

  onDestroy(() => {
    stop();
    if (tickerRef) {
      clearInterval(tickerRef);
      tickerRef = null;
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
        <p class="mono text-sm font-semibold text-fuchsia-300">${stats.price.toFixed(2)}</p>
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

