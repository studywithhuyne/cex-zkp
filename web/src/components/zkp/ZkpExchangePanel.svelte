<script lang="ts">
  import { authState } from '../../stores/authStore';

  const AUTO_REFRESH_MS = 10 * 60 * 1000; // 10 minutes

  type SolvencyResult = {
    asset: string;
    snapshot_size: number;
    root_hash: string;
    root_balance: string;
    total_liabilities: string;
    cold_wallet_assets: string;
    liabilities_leq_assets: boolean;
    verified_at: string;
  };

  let status = $state<"idle" | "loading" | "pass" | "fail" | "error">("idle");
  let errorMsg = $state("");
  let result = $state<SolvencyResult | null>(null);
  let assetFilter = $state("USDT");
  let coldWalletAssets = $state("500000000");
  let autoRefresh = $state(true);
  let countdown = $state(AUTO_REFRESH_MS / 1000);

  let intervalId: ReturnType<typeof setInterval> | null = null;
  let countdownId: ReturnType<typeof setInterval> | null = null;

  async function verifySolvency() {
    status = "loading";
    errorMsg = "";

    try {
      const res = await fetch(
        `/api/zkp/solvency?asset=${encodeURIComponent(assetFilter)}&cold_wallet_assets=${encodeURIComponent(coldWalletAssets)}`,
        { headers: { "x-user-id": ($authState.userId ?? 1).toString() } }
      );
      if (!res.ok) {
        const body = await res.json().catch(() => ({ error: `HTTP ${res.status}` }));
        throw new Error(body.error || `Server returned ${res.status}`);
      }
      const data: SolvencyResult = await res.json();
      result = data;
      status = data.liabilities_leq_assets ? "pass" : "fail";
    } catch (err: any) {
      status = "error";
      errorMsg = err.message || "Failed to verify solvency";
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    countdown = AUTO_REFRESH_MS / 1000;

    countdownId = setInterval(() => {
      countdown = Math.max(0, countdown - 1);
    }, 1000);

    intervalId = setInterval(() => {
      countdown = AUTO_REFRESH_MS / 1000;
      verifySolvency();
    }, AUTO_REFRESH_MS);
  }

  function stopAutoRefresh() {
    if (intervalId !== null) { clearInterval(intervalId); intervalId = null; }
    if (countdownId !== null) { clearInterval(countdownId); countdownId = null; }
  }

  function formatCountdown(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }

  $effect(() => {
    if (autoRefresh) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }
    return () => stopAutoRefresh();
  });
</script>

<section class="terminal-panel-strong p-4 sm:p-5 h-full flex flex-col relative overflow-hidden zk-panel">
  <div class="mb-4 flex items-start justify-between gap-3">
    <div>
      <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">
        Exchange Solvency
      </h2>
      <p class="mt-0.5 text-[10px] text-slate-500">Server-side verification</p>
    </div>
    <span class="mono rounded border border-amber-500/25 bg-amber-500/10 px-2 py-0.5 text-[10px] text-amber-200">
      Server-side
    </span>
  </div>

  <div class="flex-1 flex flex-col space-y-4">
    <!-- Controls -->
    <div class="grid gap-2 md:grid-cols-[100px_1fr_auto] md:items-center">
      <select
        bind:value={assetFilter}
        class="rounded border border-slate-700/80 bg-slate-900/80 px-2 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50 cursor-pointer"
      >
        <option value="USDT">USDT</option>
        <option value="BTC">BTC</option>
      </select>

      <input
        type="text"
        bind:value={coldWalletAssets}
        placeholder="Cold wallet assets"
        class="rounded border border-slate-700/80 bg-slate-900/80 px-2.5 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
      />

      <label class="flex items-center gap-2 cursor-pointer select-none text-xs text-slate-300">
        <input type="checkbox" bind:checked={autoRefresh} class="accent-cyan-500" />
        Auto (10m)
      </label>
    </div>

    <!-- Status board -->
    <div class="status-board p-3 rounded-xl border {
      status === 'pass'    ? 'border-emerald-500/40 text-emerald-300' :
      status === 'fail'    ? 'border-rose-500/40 text-rose-300' :
      status === 'error'   ? 'border-orange-500/40 text-orange-300' :
      status === 'loading' ? 'border-cyan-500/40 text-cyan-300' :
      'border-slate-800 text-slate-400'
    }">
      <p class="mono text-[11px] uppercase tracking-[0.15em] font-semibold">
        {#if status === 'idle'}Status: Ready
        {:else if status === 'loading'}Status: Verifying...
        {:else if status === 'pass'}Status: SOLVENT ✓
        {:else if status === 'fail'}Status: INSOLVENT ✗
        {:else if status === 'error'}Status: Error
        {/if}
      </p>

      {#if errorMsg}
        <p class="mt-2 text-xs leading-relaxed text-orange-300/95">{errorMsg}</p>
      {/if}
    </div>

    <!-- Result details -->
    {#if result}
      <div class="rounded-xl border border-slate-700/70 bg-slate-950/55 p-3 text-xs text-slate-300/90 space-y-1.5">
        <p class="mono uppercase tracking-[0.14em] text-slate-400 mb-2">Solvency Report</p>
        <p>Asset: <span class="mono text-slate-100">{result.asset}</span></p>
        <p>Total Liabilities: <span class="mono text-slate-100">{result.total_liabilities}</span></p>
        <p>Cold Wallet Assets: <span class="mono text-slate-100">{result.cold_wallet_assets}</span></p>
        <p>Root Hash: <span class="mono text-slate-100 break-all text-[10px]">{result.root_hash}</span></p>
        <p>Snapshot Size: <span class="mono text-slate-100">{result.snapshot_size} users</span></p>
        <p class="mt-1 font-semibold {result.liabilities_leq_assets ? 'text-emerald-300' : 'text-rose-300'}">
          Result: {result.liabilities_leq_assets ? 'PASS — Liabilities ≤ Assets' : 'FAIL — Liabilities > Assets'}
        </p>
        <p class="text-[10px] text-slate-500 mt-1">Verified at: {result.verified_at}</p>
      </div>
    {/if}

    <!-- Auto-refresh countdown -->
    {#if autoRefresh && status !== 'idle'}
      <div class="text-center text-[10px] text-slate-500 mono">
        Next refresh in {formatCountdown(countdown)}
      </div>
    {/if}

    <!-- Verify button -->
    <button
      type="button"
      onclick={verifySolvency}
      disabled={status === 'loading'}
      class="w-full h-10 rounded-xl border border-cyan-500/35 bg-cyan-500/18 text-sm font-semibold tracking-wider text-cyan-200 transition hover:bg-cyan-500/26 active:scale-[0.98] disabled:opacity-50 disabled:active:scale-100 uppercase"
    >
      {status === 'loading' ? 'Verifying...' : 'Verify Solvency'}
    </button>
  </div>
</section>

<style>
  .zk-panel {
    background:
      radial-gradient(740px 220px at 100% -20%, rgba(251, 191, 36, 0.07), transparent 60%),
      color-mix(in srgb, #020617 84%, transparent);
  }

  .status-board {
    background: rgba(2, 6, 23, 0.6);
  }
</style>
