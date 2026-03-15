<script lang="ts">
  import { orderBook } from "../../stores/orderBookStore";

  // Derive the maximum amount to scale the depth bars
  let maxBidAmount = $derived(
    Math.max(1, ...$orderBook.bids.slice(0, 15).map(b => b.amount))
  );
  let maxAskAmount = $derived(
    Math.max(1, ...$orderBook.asks.slice(0, 15).map(a => a.amount))
  );
</script>

<section class="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden p-0 terminal-panel-strong relative shadow-xl">
  <div class="px-3 py-2 border-b border-slate-800 flex items-center justify-between bg-slate-950/50">
    <h2 class="text-xs font-semibold tracking-widest text-slate-300 uppercase">Order Book</h2>
    <span class="mono text-[10px] text-slate-500 bg-slate-800/50 px-2 py-0.5 rounded">Real-time</span>
  </div>

  <div class="flex-1 min-h-87.5 flex overflow-hidden text-sm mono leading-relaxed tracking-tight">
    
    <!-- BIDS COLUMN (Left) -->
    <div class="flex-1 border-r border-slate-800/80 flex flex-col relative w-1/2">
      <!-- Header -->
      <div class="flex justify-between px-3 text-slate-400 font-medium tracking-widest text-[9px] uppercase border-b border-slate-800/50 py-1 bg-slate-900/80 sticky top-0 z-20">
        <span>Size</span>
        <span>Bid</span>
      </div>
      
      <!-- List -->
      <ul class="flex-1 overflow-y-auto py-1 hide-scrollbar bg-[#0f172a]">
        {#each $orderBook.bids.slice(0, 15) as bid}
          {@const width = (bid.amount / maxBidAmount) * 100}
          <li class="relative flex justify-between px-3 py-0.5 items-center group hover:bg-slate-800/50 transition-colors cursor-default select-none">
            <!-- Background Depth Bar -->
            <div 
              class="absolute top-0 bottom-0 right-0 bg-emerald-500/10 pointer-events-none" 
              style="width: {width}%;">
            </div>
            
            <span class="z-10 text-emerald-100/70 text-xs">{bid.amount.toFixed(2)}</span>
            <span class="z-10 text-emerald-400 font-bold tracking-wider text-xs">{bid.price.toFixed(2)}</span>
          </li>
        {/each}
        {#if $orderBook.bids.length === 0}
          <div class="h-full flex items-center justify-center text-slate-600 text-[10px] tracking-widest uppercase">No Bids</div>
        {/if}
      </ul>
    </div>

    <!-- ASKS COLUMN (Right) -->
    <div class="flex-1 flex flex-col relative w-1/2">
      <!-- Header -->
      <div class="flex justify-between px-3 text-slate-400 font-medium tracking-widest text-[9px] uppercase border-b border-slate-800/50 py-1 bg-slate-900/80 sticky top-0 z-20">
        <span>Ask</span>
        <span>Size</span>
      </div>

      <!-- List -->
      <ul class="flex-1 overflow-y-auto py-1 hide-scrollbar bg-[#0f172a]">
        {#if $orderBook.asks.length === 0}
          <div class="h-full flex items-center justify-center text-slate-600 text-[10px] tracking-widest uppercase">No Asks</div>
        {/if}
        {#each $orderBook.asks.slice(0, 15) as ask}
          {@const width = (ask.amount / maxAskAmount) * 100}
          <li class="relative flex justify-between px-3 py-0.5 items-center group hover:bg-slate-800/50 transition-colors cursor-default select-none">
            <!-- Background Depth Bar -->
            <div 
              class="absolute top-0 bottom-0 left-0 bg-rose-500/10 pointer-events-none" 
              style="width: {width}%;">
            </div>
            
            <span class="z-10 text-rose-400 font-bold tracking-wider text-xs">{ask.price.toFixed(2)}</span>
            <span class="z-10 text-rose-100/70 text-xs">{ask.amount.toFixed(2)}</span>
          </li>
        {/each}
      </ul>
    </div>

  </div>
</section>

<style>
  .hide-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .hide-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
