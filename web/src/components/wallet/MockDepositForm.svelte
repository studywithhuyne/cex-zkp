<script lang="ts">
  import { balanceVersion } from '../../stores/appStore';
  import { authState } from '../../stores/authStore';
  import { postDeposit, postWithdraw } from "../../lib/api/client";

  const asset = "USDT";
  let amount = $state("");
  let isSubmitting = $state(false);
  let resultMsg = $state("");
  let isError = $state(false);
  type WalletAction = "deposit" | "withdraw";

  async function handleTransfer(action: WalletAction) {
    if (!amount || parseFloat(amount) <= 0) {
      resultMsg = "Enter a valid amount";
      isError = true;
      return;
    }

    isSubmitting = true;
    resultMsg = "";
    isError = false;

    try {
      if (action === "deposit") {
        const res = await postDeposit(($authState.userId!), asset, amount);
        resultMsg = `Deposited ${res.deposited} ${res.asset}. New available: ${res.new_available}`;
      } else {
        const res = await postWithdraw(($authState.userId!), asset, amount);
        resultMsg = `Withdrew ${res.withdrawn} ${res.asset}. New available: ${res.new_available}`;
      }
      isError = false;
      amount = "";
      balanceVersion.update((v) => v + 1);
    } catch (err: any) {
      resultMsg = err.message || "Wallet transfer failed";
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section class="terminal-panel p-4 sm:p-5">
  <h2 class="mb-4 text-sm font-semibold tracking-wide text-slate-100 uppercase">Mock Wallet Transfer (USDT)</h2>

  <div class="space-y-3">
    <div>
      <label for="deposit-amount" class="block text-[10px] font-medium tracking-widest text-slate-500 uppercase mb-1">{asset} Amount</label>
      <input
        id="deposit-amount"
        type="text"
        inputmode="decimal"
        bind:value={amount}
        placeholder="10000"
        class="block w-full rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 mono outline-none transition focus:border-sky-500/50"
      />
    </div>

    <div class="grid grid-cols-2 gap-3">
      <button
        onclick={() => handleTransfer("deposit")}
        disabled={isSubmitting}
        class="w-full rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-4 py-2 text-sm font-medium text-emerald-300
          hover:bg-emerald-500/20 transition disabled:opacity-40 disabled:cursor-not-allowed"
      >
        {isSubmitting ? "Processing..." : `Deposit ${asset}`}
      </button>

      <button
        onclick={() => handleTransfer("withdraw")}
        disabled={isSubmitting}
        class="w-full rounded-lg border border-rose-500/30 bg-rose-500/10 px-4 py-2 text-sm font-medium text-rose-300
          hover:bg-rose-500/20 transition disabled:opacity-40 disabled:cursor-not-allowed"
      >
        {isSubmitting ? "Processing..." : `Withdraw ${asset}`}
      </button>
    </div>

    {#if resultMsg}
      <p class="text-xs mono {isError ? 'text-rose-400' : 'text-emerald-400'}">{resultMsg}</p>
    {/if}
  </div>
</section>

