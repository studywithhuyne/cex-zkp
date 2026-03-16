<script lang="ts">
  import { balanceVersion } from '../../stores/appStore';
  import { authState } from '../../stores/authStore';
  import { postDeposit } from "../../lib/api/client";

  let asset = $state("USDT");
  let amount = $state("");
  let isSubmitting = $state(false);
  let resultMsg = $state("");
  let isError = $state(false);

  async function handleDeposit() {
    if (!amount || parseFloat(amount) <= 0) {
      resultMsg = "Enter a valid amount";
      isError = true;
      return;
    }

    isSubmitting = true;
    resultMsg = "";
    isError = false;

    try {
      const res = await postDeposit(($authState.userId!), asset, amount);
      resultMsg = `Deposited ${res.deposited} ${res.asset}. New available: ${res.new_available}`;
      isError = false;
      amount = "";
      balanceVersion.update((v) => v + 1);
    } catch (err: any) {
      resultMsg = err.message || "Deposit failed";
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section class="terminal-panel p-4 sm:p-5">
  <h2 class="mb-4 text-sm font-semibold tracking-wide text-slate-100 uppercase">Mock Deposit</h2>

  <div class="space-y-3">
    <div class="flex gap-3">
      <div class="flex-1">
        <label for="deposit-asset" class="block text-[10px] font-medium tracking-widest text-slate-500 uppercase mb-1">Asset</label>
        <select
          id="deposit-asset"
          bind:value={asset}
          class="block w-full rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none transition focus:border-sky-500/50 cursor-pointer"
        >
          <option value="BTC">BTC</option>
          <option value="USDT">USDT</option>
        </select>
      </div>

      <div class="flex-2">
        <label for="deposit-amount" class="block text-[10px] font-medium tracking-widest text-slate-500 uppercase mb-1">Amount</label>
        <input
          id="deposit-amount"
          type="text"
          inputmode="decimal"
          bind:value={amount}
          placeholder={asset === "BTC" ? "0.5" : "10000"}
          class="block w-full rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 mono outline-none transition focus:border-sky-500/50"
        />
      </div>
    </div>

    <button
      onclick={handleDeposit}
      disabled={isSubmitting}
      class="w-full rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-4 py-2 text-sm font-medium text-emerald-300
        hover:bg-emerald-500/20 transition disabled:opacity-40 disabled:cursor-not-allowed"
    >
      {isSubmitting ? "Depositing..." : "Deposit Funds"}
    </button>

    {#if resultMsg}
      <p class="text-xs mono {isError ? 'text-rose-400' : 'text-emerald-400'}">{resultMsg}</p>
    {/if}
  </div>
</section>

