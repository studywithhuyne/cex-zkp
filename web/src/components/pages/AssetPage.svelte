<script lang="ts">
  import { balanceVersion } from "../../stores/appStore";
  import { authState } from "../../stores/authStore";
  import { router } from "../../stores/routerStore";
  import { fetchAssets, fetchBalances, postDeposit, postTransfer, postWithdraw } from "../../lib/api/client";
  import type { AssetDto, BalanceDto } from "../../lib/api/client";

  type AssetAction = "deposit" | "withdraw" | "transfer";

  type ViewAsset = {
    symbol: string;
    available: string;
    locked: string;
  };

  const WALLET_OPTIONS = ["Fiat and Spot", "Funding"];

  let allAssets = $state<AssetDto[]>([]);
  let balances = $state<BalanceDto[]>([]);
  let viewAssets = $state<ViewAsset[]>([]);
  let isLoading = $state(false);
  let action = $state<AssetAction>("deposit");
  let selectedAsset = $state("USDT");
  let amount = $state("");
  let transferAsset = $state("USDT");
  let transferAmount = $state("");
  let fromWallet = $state("Fiat and Spot");
  let toWallet = $state("Funding");
  let isSubmitting = $state(false);
  let resultMsg = $state("");
  let isError = $state(false);

  const ACTION_BUTTONS: { key: AssetAction; label: string }[] = [
    { key: "deposit", label: "Deposit" },
    { key: "withdraw", label: "Withdraw" },
    { key: "transfer", label: "Transfer" },
  ];

  // Add coin icons here: put files in web/public/icons/coins and map by symbol.
  const ASSET_ICONS: Record<string, string> = {
    BTC: "/icons/coins/btc.svg",
    ETH: "/icons/coins/eth.svg",
    SOL: "/icons/coins/sol.svg",
    BNB: "/icons/coins/bnb.svg",
    USDT: "/icons/coins/usdt.svg",
  };

  function iconPath(symbol: string): string {
    return ASSET_ICONS[symbol] ?? "/icons/coins/default.svg";
  }

  function consumeActionIntent() {
    const raw = localStorage.getItem("asset_default_action");
    if (raw === "deposit" || raw === "withdraw" || raw === "transfer") {
      action = raw;
    }
    localStorage.removeItem("asset_default_action");
  }

  function mergeAssetsAndBalances(assetList: AssetDto[], balanceList: BalanceDto[]): ViewAsset[] {
    const balanceMap = new Map(balanceList.map((b) => [b.asset, b]));
    return assetList
      .map((asset) => {
        const bal = balanceMap.get(asset.symbol);
        return {
          symbol: asset.symbol,
          available: bal?.available ?? "0",
          locked: bal?.locked ?? "0",
        };
      })
      .sort((a, b) => a.symbol.localeCompare(b.symbol));
  }

  async function loadData() {
    const userId = $authState.userId;
    if (!userId) {
      allAssets = [];
      balances = [];
      viewAssets = [];
      return;
    }

    isLoading = true;
    try {
      const [assetsFromApi, balancesFromApi] = await Promise.all([fetchAssets(), fetchBalances(userId)]);
      allAssets = assetsFromApi;
      balances = balancesFromApi;
      viewAssets = mergeAssetsAndBalances(assetsFromApi, balancesFromApi);

      if (!viewAssets.some((a) => a.symbol === selectedAsset)) {
        selectedAsset = viewAssets[0]?.symbol ?? "USDT";
      }
      if (!viewAssets.some((a) => a.symbol === transferAsset)) {
        transferAsset = viewAssets[0]?.symbol ?? "USDT";
      }
    } catch {
      allAssets = [];
      balances = [];
      viewAssets = [];
    } finally {
      isLoading = false;
    }
  }

  function availableOf(asset: string): string {
    const found = viewAssets.find((a) => a.symbol === asset);
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
      await loadData();
    } catch (err: any) {
      resultMsg = err.message || "Asset action failed";
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }

  async function submitTransfer() {
    if (fromWallet === toWallet) {
      resultMsg = "From and To wallets must be different";
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
      const res = await postTransfer(userId, {
        from_wallet: fromWallet,
        to_wallet: toWallet,
        asset: transferAsset,
        amount: transferAmount,
      });
      resultMsg = `Transferred ${res.transferred} ${transferAsset} from ${fromWallet} to ${toWallet}.`;
      transferAmount = "";
      balanceVersion.update((v) => v + 1);
      await loadData();
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
    consumeActionIntent();
    loadData();
  });
</script>

<div class="space-y-4 md:space-y-6">
  <section class="terminal-panel-strong p-6">
    <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
      <div>
        <p class="mono text-xs uppercase tracking-[0.15em] text-slate-500">Estimated Balance</p>
        <div class="mt-2 flex items-end gap-3">
          <p class="mono text-4xl font-bold text-slate-100 lg:text-[2.6rem]">
            {parseFloat(availableOf(selectedAsset)).toLocaleString(undefined, { maximumFractionDigits: 8 })}
          </p>
          <select
            bind:value={selectedAsset}
            class="rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-base text-slate-200 outline-none focus:border-sky-500/50"
          >
            {#each viewAssets as asset}
              <option value={asset.symbol}>{asset.symbol}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {#each ACTION_BUTTONS as btn}
          <button
            type="button"
            onclick={() => (action = btn.key)}
            class="rounded-lg border px-4 py-2 text-sm font-semibold transition
              {action === btn.key
                ? 'border-sky-500/50 bg-sky-500/20 text-sky-200'
                : 'border-slate-700/80 bg-slate-900/80 text-slate-300 hover:border-slate-600'}"
          >
            {btn.label}
          </button>
        {/each}
        <button
          type="button"
          onclick={openHistory}
          class="rounded-lg border border-slate-700/80 bg-slate-900/80 px-4 py-2 text-sm font-semibold text-slate-300 transition hover:border-slate-600"
        >
          History
        </button>
      </div>
    </div>

    <div class="mt-5 rounded-xl border border-slate-800/70 bg-slate-950/50 p-4">
      {#if action === "transfer"}
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">From</span>
            <select
              bind:value={fromWallet}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each WALLET_OPTIONS as wallet}
                <option value={wallet}>{wallet}</option>
              {/each}
            </select>
          </label>

          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">To</span>
            <select
              bind:value={toWallet}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each WALLET_OPTIONS as wallet}
                <option value={wallet}>{wallet}</option>
              {/each}
            </select>
          </label>

          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Coin</span>
            <select
              bind:value={transferAsset}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each viewAssets as asset}
                <option value={asset.symbol}>{asset.symbol}</option>
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
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label class="block">
            <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Asset</span>
            <select
              bind:value={selectedAsset}
              class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
            >
              {#each viewAssets as asset}
                <option value={asset.symbol}>{asset.symbol}</option>
              {/each}
            </select>
          </label>

          <label class="block">
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

  <section class="terminal-panel-strong p-5">
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-xl font-bold tracking-wide text-slate-100 uppercase">My Assets</h2>
      <span class="mono text-[12px] text-slate-500">{viewAssets.length} assets</span>
    </div>

    {#if isLoading && viewAssets.length === 0}
      <div class="flex items-center justify-center py-8 text-[10px] text-slate-500 uppercase tracking-widest animate-pulse">Loading...</div>
    {:else if viewAssets.length === 0}
      <div class="flex items-center justify-center py-8 text-[10px] text-slate-600 uppercase tracking-widest">No assets found</div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-xs sm:text-sm">
          <thead>
            <tr class="border-b border-slate-800/60 text-[11px] text-slate-500 uppercase tracking-wider">
              <th class="py-2 text-left">Coin</th>
              <th class="py-2 text-right">Available</th>
              <th class="py-2 text-right">Locked</th>
              <th class="py-2 text-right">Total</th>
            </tr>
          </thead>
          <tbody>
            {#each viewAssets as asset}
              <tr class="border-b border-slate-800/30 transition hover:bg-slate-800/20" onclick={() => (selectedAsset = asset.symbol)}>
                <td class="py-2 font-semibold text-slate-200">
                  <div class="flex items-center gap-2">
                    <img
                      src={iconPath(asset.symbol)}
                      alt={asset.symbol}
                      class="h-5 w-5 rounded-full object-cover bg-slate-800 ring-1 ring-slate-600/60"
                      loading="lazy"
                      onerror={(event) => {
                        const target = event.currentTarget as HTMLImageElement;
                        if (!target.src.endsWith('/icons/coins/default.svg')) {
                          target.src = '/icons/coins/default.svg';
                        }
                      }}
                    />
                    <span>{asset.symbol}</span>
                  </div>
                </td>
                <td class="py-2 text-right mono text-slate-200">{parseFloat(asset.available).toLocaleString(undefined, { maximumFractionDigits: 8 })}</td>
                <td class="py-2 text-right mono text-slate-400">{parseFloat(asset.locked).toLocaleString(undefined, { maximumFractionDigits: 8 })}</td>
                <td class="py-2 text-right mono text-slate-100">
                  {(parseFloat(asset.available) + parseFloat(asset.locked)).toLocaleString(undefined, { maximumFractionDigits: 8 })}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>
