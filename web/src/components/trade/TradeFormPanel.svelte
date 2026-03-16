<script lang="ts">
  import { connectionState } from '../../stores/appStore';
  import { authState } from '../../stores/authStore';
  import { orderBook } from '../../stores/orderBookStore';
  import { fetchAveragePrice, fetchBalances, type BalanceDto } from '../../lib/api/client';

  // ── Active tab ────────────────────────────────────────────────────────────
  let activeSide = $state<"buy" | "sell">("buy");

  // ── Form state ────────────────────────────────────────────────────────────
  let price  = $state("");
  let amount = $state("");
  let isSubmitting = $state(false);
  let resultMsg    = $state("");
  let isError      = $state(false);

  // ── Market price reference ────────────────────────────────────────────────
  let midPrice   = $state<string | null>(null);
  let microPrice = $state<string | null>(null);

  // ── Balances from API ─────────────────────────────────────────────────────
  let usdtAvailable = $state("0.00");
  let btcAvailable  = $state("0.000");

  async function loadAveragePrice() {
    try {
      const avg = await fetchAveragePrice("BTC_USDT");
      midPrice   = avg.mid_price;
      microPrice = avg.micro_price;
    } catch {
      midPrice   = null;
      microPrice = null;
    }
  }

  async function loadBalances() {
    if (!$authState.userId) return;
    try {
      const balances: BalanceDto[] = await fetchBalances($authState.userId);
      const usdt = balances.find(b => b.asset === "USDT");
      const btc  = balances.find(b => b.asset === "BTC");
      if (usdt) usdtAvailable = parseFloat(usdt.available).toFixed(2);
      if (btc)  btcAvailable  = parseFloat(btc.available).toFixed(3);
    } catch {
      // keep defaults on failure
    }
  }

  $effect(() => {
    void $orderBook.bids;
    void $orderBook.asks;
    void loadAveragePrice();
    void loadBalances();
  });

  // ── Computed total (price × amount) ──────────────────────────────────────
  let total = $derived(() => {
    const p = parseFloat(price);
    const a = parseFloat(amount);
    if (isNaN(p) || isNaN(a) || p <= 0 || a <= 0) return "";
    return (p * a).toFixed(2);
  });

  // ── Percentage pill handler ───────────────────────────────────────────────
  function applyPct(pct: number) {
    const p = parseFloat(price);
    if (activeSide === "buy") {
      // buy: pct of USDT balance / price → amount
      const usdt = parseFloat(usdtAvailable);
      if (!isNaN(p) && p > 0 && usdt > 0) {
        amount = ((usdt * pct / 100) / p).toFixed(6);
      }
    } else {
      // sell: pct of BTC available
      const btc = parseFloat(btcAvailable);
      if (btc > 0) {
        amount = ((btc * pct / 100)).toFixed(6);
      }
    }
  }

  // ── Submit order ──────────────────────────────────────────────────────────
  async function submitOrder() {
    if (!price || !amount) {
      resultMsg = "Price and amount are required";
      isError   = true;
      return;
    }

    isSubmitting = true;
    resultMsg    = "";
    isError      = false;
    connectionState.update(s => ({ ...s, rest: "requesting" }));

    try {
      const resp = await fetch("/api/orders", {
        method:  "POST",
        headers: {
          "Content-Type": "application/json",
          "x-user-id": ($authState.userId!).toString(),
        },
        body: JSON.stringify({
          side:        activeSide,
          price:       String(price),
          amount:      String(amount),
          base_asset:  "BTC",
          quote_asset: "USDT",
        }),
      });

      const data = await resp.json();

      if (resp.ok) {
        const matched = parseFloat(data.matched_amount ?? "0");
        resultMsg = matched > 0
          ? `Filled ${matched.toFixed(6)} BTC instantly`
          : `Order placed — resting on book`;
        isError   = false;

        // Update balances from the single response (no extra round-trip needed).
        if (Array.isArray(data.updated_balances)) {
          for (const b of data.updated_balances) {
            if (b.asset === "USDT") usdtAvailable = parseFloat(b.available).toFixed(2);
            if (b.asset === "BTC")  btcAvailable  = parseFloat(b.available).toFixed(3);
          }
        } else {
          // Fallback: fetch balances separately if server didn't include them.
          void loadBalances();
        }

        if (data.order_id) {
          window.dispatchEvent(new CustomEvent("orders:placed", {
            detail: {
              order_id:    Number(data.order_id),
              side:        activeSide,
              price:       String(price),
              amount:      String(amount),
              filled:      String(data.matched_amount ?? "0"),
              status:      matched > 0 && parseFloat(amount) === matched ? "filled" : "open",
              base_asset:  "BTC",
              quote_asset: "USDT",
              created_at:  new Date().toISOString(),
            },
          }));
        }
        window.dispatchEvent(new CustomEvent("orders:changed"));
      } else {
        resultMsg = data.error || "Failed to place order";
        isError   = true;
      }
    } catch (err: any) {
      resultMsg = err.message || "Network error";
      isError   = true;
    } finally {
      isSubmitting = false;
      connectionState.update(s => ({ ...s, rest: "ready" }));
      setTimeout(() => { if (!isSubmitting) resultMsg = ""; }, 3000);
    }
  }
</script>

<section class="flex flex-col h-full terminal-panel-strong p-4 sm:p-5 relative overflow-hidden">

  <!-- Header row: title + market badge -->
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Trade</h2>
    <span class="mono text-[10px] text-slate-500 bg-slate-800/50 px-2 py-0.5 rounded">Spot · BTC/USDT</span>
  </div>

  <!-- Mid / Micro price strip -->
  <div class="mb-4 grid grid-cols-2 gap-2 text-center">
    <div class="rounded-lg border border-slate-800 bg-slate-900/60 py-2 px-3">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Mid Price</p>
      <p class="mono text-sm font-semibold text-sky-300">{midPrice ?? "--"}</p>
    </div>
    <div class="rounded-lg border border-slate-800 bg-slate-900/60 py-2 px-3">
      <p class="text-[9px] uppercase tracking-widest text-slate-500">Micro Price</p>
      <p class="mono text-sm font-semibold text-fuchsia-300">{microPrice ?? "--"}</p>
    </div>
  </div>

  <!-- ── Buy / Sell tabs ── -->
  <div class="mb-1 grid grid-cols-2 border-b border-slate-700/60">
    <button
      type="button"
      onclick={() => activeSide = "buy"}
      class="py-2 text-sm font-semibold tracking-wide transition-colors
        {activeSide === 'buy'
          ? 'text-emerald-400 border-b-2 border-emerald-400'
          : 'text-slate-500 hover:text-slate-300 border-b-2 border-transparent'}"
    >Buy</button>
    <button
      type="button"
      onclick={() => activeSide = "sell"}
      class="py-2 text-sm font-semibold tracking-wide transition-colors
        {activeSide === 'sell'
          ? 'text-rose-400 border-b-2 border-rose-400'
          : 'text-slate-500 hover:text-slate-300 border-b-2 border-transparent'}"
    >Sell</button>
  </div>

  <!-- Contextual balance + Order type row -->
  <div class="mb-4 mt-2.5 flex items-center justify-between">
    <span class="text-[10px] uppercase tracking-wider text-slate-600 bg-slate-800/60 border border-slate-700/60 px-2 py-0.5 rounded">
      Limit
    </span>
    <span class="mono text-[11px] text-slate-400">
      Available:
      {#if activeSide === 'buy'}
        <span class="text-slate-200">{usdtAvailable} USDT</span>
      {:else}
        <span class="text-slate-200">{btcAvailable} BTC</span>
      {/if}
    </span>
  </div>

  <form class="flex-1 flex flex-col gap-3">

    <!-- PRICE -->
    <div class="space-y-1">
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="price">
        Price
      </label>
      <div class="flex items-center bg-slate-900/80 border border-slate-700/80 focus-within:border-sky-500/50 rounded-lg px-3 py-2 transition-colors">
        <input
          id="price"
          type="number"
          step="0.01"
          min="0"
          placeholder="0.00"
          bind:value={price}
          class="mono w-full bg-transparent text-sm text-slate-100 outline-none placeholder:text-slate-600"
        />
        <span class="mono text-xs text-slate-500 ml-2 shrink-0">USDT</span>
      </div>
    </div>

    <!-- AMOUNT -->
    <div class="space-y-1">
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="amount">
        Amount
      </label>
      <div class="flex items-center bg-slate-900/80 border border-slate-700/80 focus-within:border-sky-500/50 rounded-lg px-3 py-2 transition-colors">
        <input
          id="amount"
          type="number"
          step="0.001"
          min="0"
          placeholder="0.000"
          bind:value={amount}
          class="mono w-full bg-transparent text-sm text-slate-100 outline-none placeholder:text-slate-600"
        />
        <span class="mono text-xs text-slate-500 ml-2 shrink-0">BTC</span>
      </div>
    </div>

    <!-- PERCENTAGE PILLS -->
    <div class="grid grid-cols-4 gap-1.5">
      {#each [25, 50, 75, 100] as pct}
        <button
          type="button"
          onclick={() => applyPct(pct)}
          class="pct-pill mono text-[10px] font-medium py-1 rounded-full border border-slate-700/60 bg-slate-800/50 text-slate-400 transition
            hover:border-slate-500 hover:text-slate-200 hover:bg-slate-700/50 active:scale-95"
        >
          {pct}%
        </button>
      {/each}
    </div>

    <!-- TOTAL (read-only) -->
    <div class="space-y-1">
      <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="total">
        Total
      </label>
      <div class="flex items-center bg-slate-900/40 border border-slate-700/50 rounded-lg px-3 py-2">
        <span id="total" class="mono w-full text-sm {total() ? 'text-slate-200' : 'text-slate-600'}">
          {total() || "0.00"}
        </span>
        <span class="mono text-xs text-slate-500 ml-2 shrink-0">USDT</span>
      </div>
    </div>

    <!-- NOTIFICATION -->
    <div class="min-h-5">
      {#if resultMsg}
        <p class="text-[11px] mono text-center rounded px-2 py-1 {isError ? 'text-rose-400 bg-rose-500/10' : 'text-emerald-400 bg-emerald-500/10'}">
          {resultMsg}
        </p>
      {/if}
    </div>

    <!-- CTA BUTTON -->
    <button
      type="button"
      disabled={isSubmitting}
      onclick={submitOrder}
      class="cta-btn mt-auto w-full h-12 rounded-xl text-sm font-bold tracking-widest uppercase transition active:scale-[0.98] disabled:opacity-50 disabled:pointer-events-none
        {activeSide === 'buy'
          ? 'bg-emerald-500 hover:bg-emerald-400 text-white shadow-[0_4px_20px_rgba(16,185,129,0.3)]'
          : 'bg-rose-500 hover:bg-rose-400 text-white shadow-[0_4px_20px_rgba(244,63,94,0.3)]'}"
    >
      {isSubmitting ? "Placing..." : activeSide === "buy" ? "Buy BTC" : "Sell BTC"}
    </button>

  </form>
</section>

<style>
  .pct-pill {
    cursor: pointer;
  }
  .cta-btn {
    cursor: pointer;
  }
</style>
