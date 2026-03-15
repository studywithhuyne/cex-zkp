<script lang="ts">
  import { connectionState } from "../../stores/appStore.js";
</script>

<header class="terminal-panel px-4 py-4 sm:px-6">
  <div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
    <div>
      <p class="mono text-xs tracking-[0.18em] text-sky-300/80">MATCHING ENGINE / FRONTEND</p>
      <h1 class="mt-1 text-2xl font-semibold text-slate-100 sm:text-3xl">Realtime Trading Console</h1>
      <p class="mt-2 text-sm text-slate-300/80">
        UI scaffold for OrderBook, Trade API, WebSocket stream, and ZKP verifier.
      </p>
    </div>

    <div class="grid grid-cols-3 gap-2 text-xs sm:gap-3">
      <div class="rounded-xl border border-emerald-400/30 px-3 py-2
        {$connectionState.ws === 'connected' ? 'bg-emerald-500/10 text-emerald-300' : 'bg-rose-500/10 text-rose-300 border-rose-400/30'}">
        <p class="mono text-[10px] uppercase tracking-wide">WS Status</p>
        <p class="font-medium capitalize">{$connectionState.ws}</p>
      </div>
      <div class="rounded-xl border border-sky-400/30 bg-sky-500/10 px-3 py-2 text-sky-300">
        <p class="mono text-[10px] uppercase tracking-wide">Env</p>
        <p class="font-medium">Dev Net</p>
      </div>
      <div class="rounded-xl border border-fuchsia-400/30 bg-fuchsia-500/10 px-3 py-2 text-fuchsia-300">
        <p class="mono text-[10px] uppercase tracking-wide">Mode</p>
        <p class="font-medium">Dark</p>
      </div>
    </div>
  </div>
</header>
