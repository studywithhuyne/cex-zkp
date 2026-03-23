<script lang="ts">
  import { onMount } from 'svelte';
  import { balanceVersion } from '../../stores/appStore';
  import { authState } from '../../stores/authStore';
  import { fetchAssets, fetchUsers, postDeposit, postWithdraw, type UserListItem } from '../../lib/api/client';
  import { testingConfig } from '../../stores/testingStore';

  const { coldWalletAssets } = testingConfig;

  type TransferAction = "deposit" | "withdraw";
  let transferAssets = $state<string[]>(["USDT"]);

  let targetUserId = $state("1");
  let asset = $state("USDT");
  let amount = $state("");
  let isSubmitting = $state(false);
  let transferMessage = $state("");
  let isError = $state(false);
  let users = $state<UserListItem[]>([]);

  onMount(async () => {
    const currentUserId = $authState.userId;
    if (!currentUserId) {
      users = [];
      return;
    }

    try {
      const [usersData, assetsData] = await Promise.all([
        fetchUsers(currentUserId),
        fetchAssets(),
      ]);

      users = usersData;
      transferAssets = assetsData.map((a) => a.symbol);

      if (!transferAssets.includes(asset)) {
        asset = transferAssets[0] ?? "USDT";
      }

      if (users.length > 0 && !users.some((u) => u.user_id === targetUserId)) {
        const firstUser = users[0];
        if (firstUser) {
          targetUserId = firstUser.user_id;
        }
      }
    } catch {
      users = [];
      transferAssets = ["USDT"];
    }
  });

  async function handleTransfer(action: TransferAction) {
    if (!targetUserId || Number(targetUserId) <= 0) {
      transferMessage = "Enter a valid user ID";
      isError = true;
      return;
    }

    if (!amount || Number(amount) <= 0) {
      transferMessage = "Enter a valid amount";
      isError = true;
      return;
    }

    isSubmitting = true;
    transferMessage = "";
    isError = false;

    try {
      if (action === "deposit") {
        const res = await postDeposit(targetUserId, asset, amount);
        transferMessage = `Deposited ${res.deposited} ${res.asset} to user ${targetUserId}. New available: ${res.new_available}`;
      } else {
        const res = await postWithdraw(targetUserId, asset, amount);
        transferMessage = `Withdrew ${res.withdrawn} ${res.asset} from user ${targetUserId}. New available: ${res.new_available}`;
      }
      amount = "";
      isError = false;
      balanceVersion.update((v) => v + 1);
    } catch (err: any) {
      transferMessage = err.message || "Transfer failed";
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section class="terminal-panel-strong p-4 sm:p-5 h-full flex flex-col relative overflow-hidden">
  <div class="mb-4">
    <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">
      Exchange Funds Configuration
    </h2>
    <p class="mt-0.5 text-[10px] text-slate-500">
      Configure the mocked cold wallet assets for exchange solvency verification
    </p>
  </div>

  <div class="flex-1 flex flex-col space-y-4">
    <label class="block">
      <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Total Cold Wallet Assets</span>
      <input
        type="text"
        bind:value={$coldWalletAssets}
        placeholder="e.g., 500000000"
        class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
      />
    </label>
    
    <div class="mt-2 text-[10px] text-slate-400">
      Current value: <span class="font-mono text-cyan-300">{$coldWalletAssets}</span>
    </div>

    <div class="my-2 h-px bg-slate-800"></div>

    <div class="space-y-3">
      <h3 class="text-[11px] font-semibold tracking-wide text-slate-200 uppercase">User Wallet Top-up / Withdraw</h3>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <label class="block">
          <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">User</span>
          <select
            bind:value={targetUserId}
            class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
          >
            {#if users.length === 0}
              <option value={targetUserId}>User ID: {targetUserId}</option>
            {:else}
              {#each users as user}
                <option value={user.user_id}>{user.display_name} (ID: {user.user_id})</option>
              {/each}
            {/if}
          </select>
        </label>

        <label class="block">
          <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Asset</span>
          <select
            bind:value={asset}
            class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
          >
            {#each transferAssets as candidate}
              <option value={candidate}>{candidate}</option>
            {/each}
          </select>
        </label>

        <label class="block">
          <span class="mb-1 block text-[10px] font-semibold tracking-wide text-slate-400 uppercase">Amount</span>
          <input
            type="text"
            inputmode="decimal"
            bind:value={amount}
            placeholder={asset === "USDT" ? "10000" : "0.5"}
            class="w-full rounded border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50"
          />
        </label>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <button
          onclick={() => handleTransfer("deposit")}
          disabled={isSubmitting}
          class="w-full rounded border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs font-semibold text-emerald-300 transition hover:bg-emerald-500/20 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {isSubmitting ? "Processing..." : "Deposit"}
        </button>

        <button
          onclick={() => handleTransfer("withdraw")}
          disabled={isSubmitting}
          class="w-full rounded border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs font-semibold text-rose-300 transition hover:bg-rose-500/20 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {isSubmitting ? "Processing..." : "Withdraw"}
        </button>
      </div>

      {#if transferMessage}
        <p class="text-[11px] font-mono {isError ? 'text-rose-400' : 'text-emerald-400'}">{transferMessage}</p>
      {/if}
    </div>
  </div>
</section>