<script lang="ts">
  import SimulatorPanel from "../simulator/SimulatorPanel.svelte";
  import ExchangeFundsForm from "../testing/ExchangeFundsForm.svelte";
</script>

<div class="mb-4 flex items-center justify-between">
  <h1 class="font-mono text-sm tracking-widest text-slate-500 uppercase">
    Testing Environment • Multi-Market Simulator
  </h1>
  <div class="flex items-center gap-2">
    <span class="relative flex h-2 w-2">
      <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-orange-400 opacity-75"></span>
      <span class="relative inline-flex h-2 w-2 rounded-full bg-orange-500"></span>
    </span>
    <span class="font-mono text-xs tracking-wider text-orange-200">
      TEST MODE
    </span>
  </div>
</div>

<div class="grid gap-4 lg:grid-cols-2 lg:items-start md:gap-6">
  <SimulatorPanel />

  <ExchangeFundsForm />
</div>