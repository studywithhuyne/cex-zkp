<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { orderBook } from "./stores/orderBookStore.js";
  import { router } from "./stores/routerStore.js";
  import { authState, bootstrapAuth } from "./stores/authStore";
  import Navbar from "./components/layout/Navbar.svelte";
  import LoginPage from "./components/pages/LoginPage.svelte";
  import LandingPage from "./components/pages/LandingPage.svelte";
  import TradePage from "./components/pages/TradePage.svelte";
  import AssetPage from "./components/pages/AssetPage.svelte";
  import TradeHistoryPage from "./components/pages/TradeHistoryPage.svelte";
  import ZkVerifyPage from "./components/pages/ZkVerifyPage.svelte";
  import TestingPage from "./components/pages/TestingPage.svelte";
  import AdminPage from "./components/pages/AdminPage.svelte";
  import AdminLoginPage from "./components/pages/AdminLoginPage.svelte";
  import { adminAuthState, bootstrapAdminAuth } from "./stores/adminAuthStore";

  const AUTH_REQUIRED_ROUTES = new Set(["/trade", "/asset", "/trade-history", "/zk-verify", "/testing"]);

  onMount(async () => {
    orderBook.connect();
    await bootstrapAuth();
    bootstrapAdminAuth();
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

    if ($router === "/admin" && !$adminAuthState.isLoggedIn) {
      router.navigate("/admin/login");
      return;
    }

    if ($router === "/admin/login" && $adminAuthState.isLoggedIn) {
      router.navigate("/admin");
      return;
    }
  });
</script>

<div class="terminal-shell">
  {#if !$router.startsWith("/admin")}
    <Navbar />
  {/if}

  <main class="{$router.startsWith('/admin') ? 'p-4 md:p-6' : 'mt-4 md:mt-6'}">
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
    {:else if $router === "/asset"}
      <AssetPage />
    {:else if $router === "/trade-history"}
      <TradeHistoryPage />
    {:else if $router === "/zk-verify"}
      <ZkVerifyPage />
    {:else if $router === "/testing"}
      <TestingPage />
    {:else if $router === "/admin/login"}
      <AdminLoginPage />
    {:else if $router === "/admin"}
      <AdminPage />
    {/if}
  </main>
</div>
