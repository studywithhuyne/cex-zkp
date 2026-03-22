<script lang="ts">
  import { testingConfig } from '../../stores/testingStore';
  const { coldWalletAssets } = testingConfig;
</script>

<section class="terminal-panel-strong p-4 sm:p-5 h-full flex flex-col relative overflow-hidden">
  <div class="mb-4">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">
      Exchange Funds Configuration
    </h2>
    <p class="mt-0.5 text-[10px] text-slate-500">
      Configure the mocked cold wallet assets for exchange solvency verification
    </p>
  </div>

  <div class="flex-1 flex flex-col space-y-4">
    <label class="block">
      <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Total Cold Wallet Assets</span>
      <input
        type="text"
        bind:value={$coldWalletAssets}
        placeholder="e.g., 500000000"
        class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
      />
    </label>
    
    <div class="mt-2 text-[10px] text-slate-400">
      Current value: <span class="font-mono text-cyan-300">{$coldWalletAssets}</span>
    </div>
  </div>
</section>