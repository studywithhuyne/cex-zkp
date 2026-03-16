<script lang="ts">
  import { selectedUserId, connectionState } from "../../stores/appStore";

  let price = $state("");
  let amount = $state("");
  let isSubmitting = $state(false);
  let resultMsg = $state("");
  let isError = $state(false);

  async function submitOrder(side: "buy" | "sell") {
    if (!price || !amount) {
      resultMsg = "Price and amount are required";
      isError = true;
      return;
    }

    isSubmitting = true;
    resultMsg = "";
    isError = false;
    connectionState.update(s => ({ ...s, rest: "requesting" }));

    try {
      const resp = await fetch("/api/orders", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-user-id": $selectedUserId.toString()
        },
        body: JSON.stringify({
          side,
          price: String(price),
          amount: String(amount),
          base_asset: "BTC",
          quote_asset: "USDT"
        })
      });

      const data = await resp.json();

      if (resp.ok) {
        resultMsg = `Success! Order ID: ${data.order_id || 'Matched'}`;
        isError = false;
        window.dispatchEvent(new CustomEvent("orders:changed"));
        // Optionally clear form
        // price = "";
        // amount = "";
      } else {
        resultMsg = data.error || "Failed to place order";
        isError = true;
      }
    } catch (err: any) {
      resultMsg = err.message || "Network error";
      isError = true;
    } finally {
      isSubmitting = false;
      connectionState.update(s => ({ ...s, rest: "ready" }));
      setTimeout(() => { if (!isSubmitting) resultMsg = ""; }, 3000);
    }
  }
</script>

<section class="flex flex-col h-full terminal-panel-strong p-4 sm:p-5 relative overflow-hidden">
  <div class="mb-5 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Trade Form</h2>
    <span class="mono text-[10px] text-slate-500 bg-slate-800/50 px-2 py-0.5 rounded">Spot  BTC/USDT</span>
  </div>

  <form class="flex-1 flex flex-col space-y-4">
    <!-- PRICE -->
    <div class="space-y-1">
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="price">Price (USDT)</label>
      <div class="flex items-center bg-slate-900/80 border border-slate-700/80 focus-within:border-sky-500/50 rounded-lg px-3 py-2 transition-colors">
        <input
          id="price"
          type="number"
          step="0.01"
          placeholder="0.00"
          bind:value={price}
          class="mono w-full bg-transparent text-sm text-slate-100 outline-none placeholder:text-slate-600"
        />
        <span class="mono text-xs text-slate-500 ml-2">USDT</span>
      </div>
    </div>

    <!-- AMOUNT -->
    <div class="space-y-1">
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="amount">Amount (BTC)</label>
      <div class="flex items-center bg-slate-900/80 border border-slate-700/80 focus-within:border-sky-500/50 rounded-lg px-3 py-2 transition-colors">
        <input
          id="amount"
          type="number"
          step="0.001"
          placeholder="0.000"
          bind:value={amount}
          class="mono w-full bg-transparent text-sm text-slate-100 outline-none placeholder:text-slate-600"
        />
        <span class="mono text-xs text-slate-500 ml-2">BTC</span>
      </div>
    </div>

    <div class="pt-2 flex-col flex gap-3 h-full justify-end">
      <!-- NOTIFICATION AREA -->
      <div class="h-6">
        {#if resultMsg}
          <div class="text-[11px] mono text-center rounded p-1 {isError ? 'text-rose-400 bg-rose-500/10' : 'text-emerald-400 bg-emerald-500/10'}">
            {resultMsg}
          </div>
        {/if}
      </div>

      <!-- BUTTONS -->
      <div class="grid grid-cols-2 gap-3">
        <button
          type="button"
          disabled={isSubmitting}
          onclick={() => submitOrder("buy")}
          class="h-10 rounded-lg border border-emerald-500/30 bg-emerald-500/20 text-sm font-semibold tracking-wider text-emerald-300 transition hover:bg-emerald-500/30 active:scale-[0.98] disabled:opacity-50 disabled:pointer-events-none uppercase"
        >
          Buy
        </button>
        <button
          type="button"
          disabled={isSubmitting}
          onclick={() => submitOrder("sell")}
          class="h-10 rounded-lg border border-rose-500/30 bg-rose-500/20 text-sm font-semibold tracking-wider text-rose-300 transition hover:bg-rose-500/30 active:scale-[0.98] disabled:opacity-50 disabled:pointer-events-none uppercase"
        >
          Sell
        </button>
      </div>
    </div>
  </form>
</section>
