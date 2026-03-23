<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { selectedMarket } from "../../stores/marketStore";

  // Pre-defined list of assets against USDT
  const ASSETS = ["BTC", "ETH", "SOL", "BNB"];

  // State to hold current prices and 24h change from Binance API
  type TickerData = { price: string; pChange: string; isUp: boolean };
  let tickers = $state<Record<string, TickerData>>({});

  let intervalId: any;

  async function fetchTickers() {
    try {
      // Binance 24hr ticker endpoint
      const symbols = ASSETS.map(a => `"${a}USDT"`).join(",");
      const res = await fetch(`https://api.binance.com/api/v3/ticker/24hr?symbols=[${symbols}]`);
      if (res.ok) {
        const data = await res.json();
        let newTickers: Record<string, TickerData> = {};
        for (const item of data) {
          const base = item.symbol.replace("USDT", "");
          const price = parseFloat(item.lastPrice);
          const pChange = parseFloat(item.priceChangePercent);
          
          newTickers[base] = {
            price: price < 10 ? price.toFixed(4) : price.toFixed(2),
            pChange: Math.abs(pChange).toFixed(2),
            isUp: pChange >= 0
          };
        }
        tickers = newTickers;
      }
    } catch (err) {
      console.error("Failed to fetch tickers", err);
    }
  }

  onMount(() => {
    fetchTickers();
    intervalId = setInterval(fetchTickers, 3000); // refresh every 3s
  });

  onDestroy(() => {
    if (intervalId) clearInterval(intervalId);
  });

  function selectMarket(base: string) {
    if ($selectedMarket !== `${base}_USDT`) {
      selectedMarket.set(`${base}_USDT`);
    }
  }
</script>

<div class="flex overflow-x-auto bg-[#0A0E17] border-b border-slate-800/50 p-2 gap-2 text-xs font-mono custom-scrollbar mb-4 rounded-xl">
  {#each ASSETS as asset}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="flex items-center gap-3 px-4 py-2 rounded-lg cursor-pointer transition-colors border {$selectedMarket === `${asset}_USDT` ? 'border-sky-500 bg-slate-800' : 'border-slate-800 hover:bg-slate-800/50'}"
      onclick={() => selectMarket(asset)}
    >
      <span class="text-slate-300 font-bold">{asset}/USDT</span>
      {#if tickers[asset]}
        <span class="font-medium {tickers[asset].isUp ? 'text-emerald-400' : 'text-rose-400'}">
          ${tickers[asset].price}
        </span>
        <span class="text-[10px] px-1 rounded {tickers[asset].isUp ? 'bg-emerald-500/20 text-emerald-300' : 'bg-rose-500/20 text-rose-300'}">
          {tickers[asset].isUp ? '+' : '-'}{tickers[asset].pChange}%
        </span>
      {:else}
        <span class="text-slate-500 animate-pulse">Loading...</span>
      {/if}
    </div>
  {/each}
</div>

<style>
.custom-scrollbar::-webkit-scrollbar {
  height: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1e293b;
  border-radius: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #334155;
}
</style>