<script lang="ts">
  import WalletBalances from "../wallet/WalletBalances.svelte";
  import MockDepositForm from "../wallet/MockDepositForm.svelte";
  import TradeHistoryPanel from "../wallet/TradeHistoryPanel.svelte";
</script>

<div class="grid gap-4 lg:grid-cols-[1fr_1fr] lg:items-start md:gap-6">
  <div class="space-y-4 md:space-y-6">
    <WalletBalances />
    <MockDepositForm />
  </div>
  <TradeHistoryPanel />
</div>

