<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { orderBook } from "./stores/orderBookStore.js";
  import { router } from "./stores/routerStore.js";
  import { authState, bootstrapAuth } from "./stores/authStore";
  import Navbar from "./components/layout/Navbar.svelte";
  import LoginPage from "./components/pages/LoginPage.svelte";
  import LandingPage from "./components/pages/LandingPage.svelte";
  import TradePage from "./components/pages/TradePage.svelte";
  import WalletPage from "./components/pages/WalletPage.svelte";
  import ZkVerifyPage from "./components/pages/ZkVerifyPage.svelte";
  import TestingPage from "./components/pages/TestingPage.svelte";

  const AUTH_REQUIRED_ROUTES = new Set(["/trade", "/wallet", "/zk-verify", "/testing"]);

  onMount(async () => {
    orderBook.connect();
    await bootstrapAuth();
  });

  onDestroy(() => {
    orderBook.disconnect();
  });

  $effect(() => {
    if ($authState.loading) {
      return;
    }

    if (!$authState.userId && AUTH_REQUIRED_ROUTES.has($router)) {
      router.navigate("/login");
      return;
    }

    if ($authState.userId && $router === "/login") {
      router.navigate("/trade");
    }
  });
</script>

<div class="terminal-shell">
  <Navbar />

  <main class="mt-4 md:mt-6">
    {#if $authState.loading}
      <section class="terminal-panel-strong p-6 text-center text-sm text-slate-300">
        Initializing session...
      </section>
    {:else if $router === "/"}
      <LandingPage />
    {:else if $router === "/login"}
      <LoginPage />
    {:else if $router === "/trade"}
      <TradePage />
    {:else if $router === "/wallet"}
      <WalletPage />
    {:else if $router === "/zk-verify"}
      <ZkVerifyPage />
    {:else if $router === "/testing"}
      <TestingPage />
    {/if}
  </main>
</div>
