<section class="terminal-panel-strong p-4 sm:p-5">
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-lg font-semibold text-slate-100">Trade Form</h2>
    <span class="mono text-xs text-slate-400">POST /api/orders</span>
  </div>

  <form class="space-y-3">
    <label class="block text-sm text-slate-300" for="price">
      Price
      <input
        id="price"
        type="text"
        placeholder="100.00"
        class="mono mt-2 block w-full rounded-xl border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none transition focus:border-sky-400"
      />
    </label>

    <label class="block text-sm text-slate-300" for="amount">
      Amount
      <input
        id="amount"
        type="text"
        placeholder="0.5000"
        class="mono mt-2 block w-full rounded-xl border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none transition focus:border-sky-400"
      />
    </label>

    <div class="grid grid-cols-2 gap-3 pt-1">
      <button
        type="button"
        class="h-10 rounded-xl border border-emerald-400/40 bg-emerald-500/15 text-sm font-medium text-emerald-200 transition hover:bg-emerald-500/25"
      >
        Buy
      </button>
      <button
        type="button"
        class="h-10 rounded-xl border border-rose-400/40 bg-rose-500/15 text-sm font-medium text-rose-200 transition hover:bg-rose-500/25"
      >
        Sell
      </button>
    </div>
  </form>
</section>
