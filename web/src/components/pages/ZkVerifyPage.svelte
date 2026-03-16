<script lang="ts">
  import ZkpVerifierPanel from "../zkp/ZkpVerifierPanel.svelte";
</script>

<section class="zk-workspace">
  <div class="grid gap-4 md:gap-6 xl:grid-cols-2 xl:items-start">
    <ZkpVerifierPanel mode="user" />
    <ZkpVerifierPanel mode="exchange" />
  </div>
</section>

<style>
  .zk-workspace {
    animation: fade-up 320ms ease-out both;
  }
</style>

