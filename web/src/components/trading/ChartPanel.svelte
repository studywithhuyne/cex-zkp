<script lang="ts">
  import TradingChart from "./TradingChart.svelte";
</script>

<section class="flex flex-col w-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden p-0 terminal-panel-strong relative shadow-xl">
  <div class="px-3 py-2 border-b border-slate-800 flex items-center justify-between bg-slate-950/50">
    <div class="flex items-center gap-3">
      <h2 class="text-xs font-semibold tracking-widest text-slate-300 uppercase">Chart</h2>
      <span class="text-[10px] font-bold text-slate-400 bg-slate-800 px-2 py-0.5 rounded">BTC/USDT</span>
    </div>
    <span class="mono text-[10px] text-slate-500 bg-slate-800/50 px-2 py-0.5 rounded">Spot</span>
  </div>

  <div class="flex-1 w-full bg-[#0A0E17]">
    <TradingChart />
  </div>
</section>
