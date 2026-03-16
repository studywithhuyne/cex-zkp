<script lang="ts">
  import { orderBook } from "../../stores/orderBookStore";

  // ── Simulator state ────────────────────────────────────────────────────────
  let isRunning   = $state(false);
  let speedKey    = $state<"slow" | "normal" | "fast">("normal");
  let totalOrders = $state(0);
  let totalFills  = $state(0);
  let intervalRef: ReturnType<typeof setInterval> | null = null;

  // Tracks last-traded price; seeds the random walk
  let basePrice = $state(65000);

  // Update basePrice whenever a new trade arrives via WebSocket
  $effect(() => {
    const latest = $orderBook.trades[0]?.price;
    if (latest && latest > 0) basePrice = latest;
  });

  // ── Speed presets (ms between orders) ─────────────────────────────────────
  const SPEEDS = { slow: 1800, normal: 900, fast: 350 } as const;

  // ── Price generation ───────────────────────────────────────────────────────
  function rand(min: number, max: number) {
    return min + Math.random() * (max - min);
  }

  function generatePrice(side: "buy" | "sell", aggressive: boolean): number {
    const spread = basePrice * 0.002; // 0.2 % half-spread
    if (side === "buy") {
      // Aggressive buy: above mid (fills existing asks)
      // Passive buy:    below mid (rests on bid side)
      return aggressive
        ? basePrice + rand(0, spread * 0.4)
        : basePrice - rand(spread, spread * 4);
    } else {
      // Aggressive sell: below mid (fills existing bids)
      // Passive sell:    above mid (rests on ask side)
      return aggressive
        ? basePrice - rand(0, spread * 0.4)
        : basePrice + rand(spread, spread * 4);
    }
  }

  // ── Single order tick ──────────────────────────────────────────────────────
  async function tick() {
    const side       = Math.random() < 0.5 ? "buy" : "sell";
    const aggressive = Math.random() < 0.35; // 35 % chance to cross the spread
    const price      = Math.max(0.01, generatePrice(side, aggressive));
    const amount     = rand(0.001, 0.06).toFixed(4);
    const userId     = Math.floor(Math.random() * 4) + 1;

    try {
      const resp = await fetch("/api/orders", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-user-id": String(userId),
        },
        body: JSON.stringify({
          side,
          price:       price.toFixed(2),
          amount,
          base_asset:  "BTC",
          quote_asset: "USDT",
        }),
      });

      if (resp.ok) {
        totalOrders++;
        const data = await resp.json();
        if ((data.trades_count ?? 0) > 0) totalFills += data.trades_count;
      }
    } catch {
      // Silently ignore network hiccups; simulation continues.
    }
  }

  // ── Controls ───────────────────────────────────────────────────────────────
  function start() {
    if (isRunning) return;
    isRunning   = true;
    intervalRef = setInterval(tick, SPEEDS[speedKey]);
  }

  function stop() {
    if (!isRunning) return;
    isRunning = false;
    if (intervalRef) { clearInterval(intervalRef); intervalRef = null; }
  }

  function toggle() { isRunning ? stop() : start(); }

  function setSpeed(k: "slow" | "normal" | "fast") {
    speedKey = k;
    if (isRunning) { stop(); start(); }
  }

  function reset() {
    stop();
    totalOrders = 0;
    totalFills  = 0;
  }
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
    <span class="mono text-[10px] text-slate-500 bg-slate-800/50 px-2 py-0.5 rounded">Auto-trader</span>
  </div>

  <!-- Last price -->
  <div class="mb-4 rounded-lg border border-slate-800 bg-slate-900/60 py-2.5 px-3 text-center">
    <p class="text-[9px] uppercase tracking-widest text-slate-500">Market Price</p>
    <p class="mt-0.5 font-bold mono text-lg text-fuchsia-300">
      {basePrice.toFixed(2)}
      <span class="text-xs text-slate-500">USDT</span>
    </p>
  </div>

  <!-- Stats -->
  <div class="mb-4 grid grid-cols-2 gap-2 text-center">
    <div class="rounded-lg border border-slate-800 bg-slate-900/50 py-2">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Orders</p>
      <p class="text-lg font-bold mono text-slate-200">{totalOrders}</p>
    </div>
    <div class="rounded-lg border border-slate-800 bg-slate-900/50 py-2">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Fills</p>
      <p class="text-lg font-bold mono text-emerald-400">{totalFills}</p>
    </div>
  </div>

  <!-- Speed selector -->
  <div class="mb-4">
    <p class="mb-1.5 text-[10px] uppercase tracking-widest text-slate-500">Speed</p>
    <div class="grid grid-cols-3 gap-1.5">
      {#each (["slow", "normal", "fast"] as const) as s}
        <button
          type="button"
          onclick={() => setSpeed(s)}
          class="rounded-md border py-1 text-[11px] font-medium uppercase tracking-wide transition
            {speedKey === s
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
</section>

