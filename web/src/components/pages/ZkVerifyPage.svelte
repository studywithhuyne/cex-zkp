<script lang="ts">
  import ZkpVerifierPanel from "../zkp/ZkpVerifierPanel.svelte";
  import ZkpExchangePanel from "../zkp/ZkpExchangePanel.svelte";
</script>

<section class="zk-workspace">
  <div class="grid gap-4 md:gap-6 xl:grid-cols-2 xl:items-start">
    <ZkpVerifierPanel />
    <ZkpExchangePanel />
  </div>
</section>

<style>
  .zk-workspace {
    animation: fade-up 320ms ease-out both;
  }
</style>

