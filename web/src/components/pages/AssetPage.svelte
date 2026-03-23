<script lang="ts">
  import { balanceVersion } from "../../stores/appStore";
  import { authState } from "../../stores/authStore";
  import { router } from "../../stores/routerStore";
  import { fetchBalances, postDeposit, postTransfer, postWithdraw } from "../../lib/api/client";
  import type { BalanceDto } from "../../lib/api/client";

  type AssetAction = "deposit" | "withdraw" | "transfer";

  let balances = $state<BalanceDto[]>([]);
  let isLoading = $state(false);
  let action = $state<AssetAction>("deposit");
  let selectedAsset = $state("USDT");
  let amount = $state("");
  let transferFrom = $state("USDT");
  let transferTo = $state("BTC");
  let transferAmount = $state("");
  let isSubmitting = $state(false);
  let resultMsg = $state("");
  let isError = $state(false);

  const ACTION_BUTTONS: { key: AssetAction; label: string }[] = [
    { key: "deposit", label: "Deposit" },
    { key: "withdraw", label: "Withdraw" },
    { key: "transfer", label: "Transfer" },
  ];

  async function loadBalances() {
    const userId = $authState.userId;
    if (!userId) {
      balances = [];
      return;
    }

    isLoading = true;
    try {
      const next = await fetchBalances(userId);
      balances = next;

      if (!next.some((b) => b.asset === selectedAsset)) {
        selectedAsset = next[0]?.asset ?? "USDT";
      }

      if (!next.some((b) => b.asset === transferFrom)) {
        transferFrom = next[0]?.asset ?? "USDT";
      }

      if (!next.some((b) => b.asset === transferTo) || transferTo === transferFrom) {
        transferTo = next.find((b) => b.asset !== transferFrom)?.asset ?? transferFrom;
      }
    } catch {
      balances = [];
    } finally {
      isLoading = false;
    }
  }

  function availableOf(asset: string): string {
    const found = balances.find((b) => b.asset === asset);
    return found?.available ?? "0";
  }

  async function submitDepositOrWithdraw(nextAction: "deposit" | "withdraw") {
    if (!amount || Number(amount) <= 0) {
      resultMsg = "Enter a valid amount";
      isError = true;
      return;
    }

    const userId = $authState.userId;
    if (!userId) return;

    isSubmitting = true;
    resultMsg = "";
    isError = false;

    try {
      if (nextAction === "deposit") {
        const res = await postDeposit(userId, selectedAsset, amount);
        resultMsg = `Deposited ${res.deposited} ${res.asset}. New available: ${res.new_available}`;
      } else {
        const res = await postWithdraw(userId, selectedAsset, amount);
        resultMsg = `Withdrew ${res.withdrawn} ${res.asset}. New available: ${res.new_available}`;
      }
      amount = "";
      balanceVersion.update((v) => v + 1);
      await loadBalances();
    } catch (err: any) {
      resultMsg = err.message || "Asset action failed";
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }

  async function submitTransfer() {
    if (transferFrom === transferTo) {
      resultMsg = "From and To assets must be different";
      isError = true;
      return;
    }
    if (!transferAmount || Number(transferAmount) <= 0) {
      resultMsg = "Enter a valid transfer amount";
      isError = true;
      return;
    }

    const userId = $authState.userId;
    if (!userId) return;

    isSubmitting = true;
    resultMsg = "";
    isError = false;

    try {
      const res = await postTransfer(userId, transferFrom, transferTo, transferAmount);
      resultMsg = `Transferred ${res.transferred} ${res.from_asset} -> ${res.to_asset}.`;
      transferAmount = "";
      balanceVersion.update((v) => v + 1);
      await loadBalances();
    } catch (err: any) {
      resultMsg = err.message || "Transfer failed";
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }

  function openHistory() {
    router.navigate("/trade-history");
  }

  $effect(() => {
    void $authState.userId;
    void $balanceVersion;
    loadBalances();
  });
</script>

<div class="space-y-4 md:space-y-6">
  <section class="terminal-panel-strong p-4 sm:p-5">
    <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
      <div>
        <p class="text-[11px] uppercase tracking-widest text-slate-500">Estimated Balance</p>
        <div class="mt-2 flex items-end gap-2">
          <p class="mono text-4xl font-bold text-slate-100">
            {parseFloat(availableOf(selectedAsset)).toLocaleString(undefined, { maximumFractionDigits: 8 })}
          </p>
          <select
            bind:value={selectedAsset}
            class="rounded border border-slate-700/80 bg-slate-900/80 px-2 py-1 text-sm text-slate-200 outline-none focus:border-sky-500/50"
          >
            {#each balances as bal}
              <option value={bal.asset}>{bal.asset}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {#each ACTION_BUTTONS as btn}
          <button
            type="button"
            onclick={() => (action = btn.key)}
            class="rounded-lg border px-3 py-2 text-sm font-semibold transition
              {action === btn.key
                ? 'border-sky-500/40 bg-sky-500/20 text-sky-200'
                : 'border-slate-700/80 bg-slate-900/80 text-slate-300 hover:border-slate-600'}"
          >
            {btn.label}
          </button>
        {/each}

        <button
          type="button"
          onclick={openHistory}
          class="rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm font-semibold text-slate-300 transition hover:border-slate-600"
        >
          History
        </button>
      </div>
    </div>

    <div class="mt-4 rounded-lg border border-slate-800/70 bg-slate-950/50 p-3">
      {#if action === "transfer"}
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">From</span>
            <select
              bind:value={transferFrom}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each balances as bal}
                <option value={bal.asset}>{bal.asset}</option>
              {/each}
            </select>
          </label>

          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">To</span>
            <select
              bind:value={transferTo}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each balances as bal}
                <option value={bal.asset}>{bal.asset}</option>
              {/each}
            </select>
          </label>

          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Amount</span>
            <input
              type="text"
              inputmode="decimal"
              bind:value={transferAmount}
              placeholder="100"
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            />
          </label>
        </div>

        <button
          type="button"
          onclick={submitTransfer}
          disabled={isSubmitting}
          class="mt-3 rounded-lg border border-fuchsia-500/30 bg-fuchsia-500/10 px-4 py-2 text-sm font-semibold text-fuchsia-300 transition hover:bg-fuchsia-500/20 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {isSubmitting ? "Processing..." : "Transfer"}
        </button>
      {:else}
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Asset</span>
            <select
              bind:value={selectedAsset}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each balances as bal}
                <option value={bal.asset}>{bal.asset}</option>
              {/each}
            </select>
          </label>

          <label class="block sm:col-span-2">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Amount</span>
            <input
              type="text"
              inputmode="decimal"
              bind:value={amount}
              placeholder="1000"
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            />
          </label>
        </div>

        <button
          type="button"
          onclick={() => submitDepositOrWithdraw(action === 'deposit' ? 'deposit' : 'withdraw')}
          disabled={isSubmitting}
          class="mt-3 rounded-lg border px-4 py-2 text-sm font-semibold transition disabled:cursor-not-allowed disabled:opacity-40
            {action === 'deposit'
              ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300 hover:bg-emerald-500/20'
              : 'border-rose-500/30 bg-rose-500/10 text-rose-300 hover:bg-rose-500/20'}"
        >
          {#if isSubmitting}
            Processing...
          {:else if action === "deposit"}
            Deposit
          {:else}
            Withdraw
          {/if}
        </button>
      {/if}

      {#if resultMsg}
        <p class="mt-3 text-[11px] font-mono {isError ? 'text-rose-400' : 'text-emerald-400'}">{resultMsg}</p>
      {/if}
    </div>
  </section>

  <section class="terminal-panel-strong p-4 sm:p-5">
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">My Assets</h2>
      <span class="mono text-[10px] text-slate-500">{balances.length} assets</span>
    </div>

    {#if isLoading && balances.length === 0}
      <div class="flex items-center justify-center py-8 text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
    {:else if balances.length === 0}
      <div class="flex items-center justify-center py-8 text-[10px] text-slate-600 uppercase tracking-widest">No balances found</div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="border-b border-slate-800/60 text-[10px] text-slate-500 uppercase tracking-wider">
              <th class="py-2 text-left">Coin</th>
              <th class="py-2 text-right">Available</th>
              <th class="py-2 text-right">Locked</th>
              <th class="py-2 text-right">Total</th>
            </tr>
          </thead>
          <tbody>
            {#each balances as bal}
              <tr
                class="cursor-pointer border-b border-slate-800/30 transition hover:bg-slate-800/20"
                onclick={() => (selectedAsset = bal.asset)}
              >
                <td class="py-2 font-semibold text-slate-200">{bal.asset}</td>
                <td class="py-2 text-right mono text-slate-200">{parseFloat(bal.available).toLocaleString(undefined, { maximumFractionDigits: 8 })}</td>
                <td class="py-2 text-right mono text-slate-400">{parseFloat(bal.locked).toLocaleString(undefined, { maximumFractionDigits: 8 })}</td>
                <td class="py-2 text-right mono text-slate-100">
                  {(parseFloat(bal.available) + parseFloat(bal.locked)).toLocaleString(undefined, { maximumFractionDigits: 8 })}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>
