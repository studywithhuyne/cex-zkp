<script lang="ts">
  import { authState } from '../../stores/authStore';
  import { orderBook } from "../../stores/orderBookStore";
  import { fetchOpenOrders, cancelOrder } from "../../lib/api/client";
  import type { OpenOrder } from "../../lib/api/client";

  let orders = $state<OpenOrder[]>([]);
  let isLoading = $state(false);
  let loadError = $state("");
  let cancellingId = $state<number | null>(null);
  let tableContainer = $state<HTMLDivElement | null>(null);
  let reloadTimer = $state<ReturnType<typeof setTimeout> | null>(null);
  let pollTimer = $state<ReturnType<typeof setInterval> | null>(null);
  let lastTopOrderId = $state<number | null>(null);

  async function loadOrders() {
    const userId = $authState.userId;
    if (!userId) {
      orders = [];
      loadError = "";
      return;
    }

    isLoading = true;
    loadError = "";
    try {
      const nextOrders = await fetchOpenOrders(userId);
      const nextTopId = nextOrders[0]?.order_id ?? null;
      const hasNewTop = lastTopOrderId !== null && nextTopId !== null && nextTopId !== lastTopOrderId;

      orders = nextOrders;
      lastTopOrderId = nextTopId;

      if (hasNewTop && tableContainer) {
        tableContainer.scrollTo({ top: 0, behavior: "smooth" });
      }
    } catch (err: any) {
      orders = [];
      loadError = err?.message ?? "Failed to load open orders";
    } finally {
      isLoading = false;
    }
  }

  function scheduleReload() {
    if (reloadTimer) return;

    reloadTimer = setTimeout(() => {
      reloadTimer = null;
      void loadOrders();
    }, 120);
  }

  $effect(() => {
    void $authState.userId;
    void loadOrders();
    scheduleReload();
  });

  $effect(() => {
    // orderbook_update arrives for every place/cancel/match, even when no trade is executed.
    void $orderBook.bids;
    void $orderBook.asks;
    scheduleReload();
  });

  $effect(() => {
    const onOrderPlaced = (event: Event) => {
      const custom = event as CustomEvent<OpenOrder>;
      const incoming = custom.detail;
      if (!incoming || incoming.order_id == null) {
        return;
      }

      const exists = orders.some((order) => order.order_id === incoming.order_id);
      if (!exists) {
        orders = [incoming, ...orders];
        lastTopOrderId = incoming.order_id;
      }
      scheduleReload();
    };

    const onOrdersChanged = () => {
      scheduleReload();
    };

    window.addEventListener("orders:placed", onOrderPlaced as EventListener);
    window.addEventListener("orders:changed", onOrdersChanged);
    pollTimer = setInterval(() => {
      scheduleReload();
    }, 1500);

    return () => {
      window.removeEventListener("orders:placed", onOrderPlaced as EventListener);
      window.removeEventListener("orders:changed", onOrdersChanged);
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
      if (reloadTimer) {
        clearTimeout(reloadTimer);
      }
    };
  });

  async function handleCancel(orderId: number) {
    cancellingId = orderId;
    try {
      await cancelOrder(($authState.userId!), orderId);
      orders = orders.filter((o) => o.order_id !== orderId);
      window.dispatchEvent(new CustomEvent("orders:changed"));
    } catch (err: any) {
      console.error("Cancel failed:", err.message);
    } finally {
      cancellingId = null;
    }
  }
</script>

<section class="terminal-panel p-4 sm:p-5 h-105 sm:h-115 flex flex-col">
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">Open Orders</h2>
    <span class="mono text-[10px] text-slate-500">{orders.length} order{orders.length !== 1 ? "s" : ""}</span>
  </div>

  <div class="flex-1 min-h-0">
    {#if loadError}
      <div class="h-full flex items-center justify-center py-6 px-3 text-[10px] text-orange-400 uppercase tracking-widest text-center">
        {loadError}
      </div>
    {:else if isLoading && orders.length === 0}
      <div class="h-full flex items-center justify-center py-6 text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
    {:else if orders.length === 0}
      <div class="h-full flex items-center justify-center py-6 text-[10px] text-slate-600 uppercase tracking-widest">No open orders</div>
    {:else}
      <div class="h-full overflow-auto" bind:this={tableContainer}>
      <table class="w-full text-xs">
        <thead class="sticky top-0 z-10 bg-slate-950/90 backdrop-blur-sm">
          <tr class="border-b border-slate-800/60 text-[10px] text-slate-500 uppercase tracking-wider">
            <th class="py-1.5 text-left font-medium">ID</th>
            <th class="py-1.5 text-left font-medium">Side</th>
            <th class="py-1.5 text-right font-medium">Price</th>
            <th class="py-1.5 text-right font-medium">Amount</th>
            <th class="py-1.5 text-right font-medium">Filled</th>
            <th class="py-1.5 text-center font-medium">Status</th>
            <th class="py-1.5 text-right font-medium"></th>
          </tr>
        </thead>
        <tbody>
          {#each orders as order}
            <tr class="border-b border-slate-800/30 hover:bg-slate-800/20 transition">
              <td class="py-1.5 mono text-slate-400">#{order.order_id}</td>
              <td class="py-1.5 font-medium uppercase {order.side === 'buy' ? 'text-emerald-400' : 'text-rose-400'}">
                {order.side}
              </td>
              <td class="py-1.5 text-right mono text-slate-200">{parseFloat(order.price).toLocaleString()}</td>
              <td class="py-1.5 text-right mono text-slate-200">{order.amount}</td>
              <td class="py-1.5 text-right mono text-slate-400">{order.filled}</td>
              <td class="py-1.5 text-center">
                <span class="rounded-full px-1.5 py-0.5 text-[9px] uppercase tracking-wider
                  {order.status === 'partial' ? 'bg-amber-500/15 text-amber-300' : 'bg-sky-500/15 text-sky-300'}">
                  {order.status}
                </span>
              </td>
              <td class="py-1.5 text-right">
                <button
                  onclick={() => handleCancel(order.order_id)}
                  disabled={cancellingId === order.order_id}
                  class="rounded px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider
                    bg-rose-500/10 text-rose-400 border border-rose-500/20
                    hover:bg-rose-500/20 transition disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  {cancellingId === order.order_id ? "..." : "Cancel"}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      </div>
    {/if}
  </div>
</section>

