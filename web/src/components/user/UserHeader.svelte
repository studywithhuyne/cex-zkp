<section class="terminal-panel-strong p-4 sm:p-5">
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-lg font-semibold text-slate-100">User Context</h2>
    <span class="mono rounded-lg border border-slate-700/80 bg-slate-900/70 px-2 py-1 text-xs text-slate-300">
      Header: x-user-id
    </span>
  </div>

  <div class="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-end">
    <label class="text-sm text-slate-300" for="user-id">
      Mock user for API calls
      <select
        id="user-id"
        class="mt-2 block w-full rounded-xl border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none ring-0 transition focus:border-sky-400"
      >
        <option value="1">Alice (1)</option>
        <option value="2">Bob (2)</option>
        <option value="3">Carol (3)</option>
      </select>
    </label>

    <button
      type="button"
      class="h-10 rounded-xl border border-sky-400/40 bg-sky-500/15 px-4 text-sm font-medium text-sky-200 transition hover:bg-sky-500/20"
    >
      Apply User
    </button>
  </div>
</section>
