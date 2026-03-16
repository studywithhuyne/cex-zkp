<script lang="ts">
  import { router } from "../../stores/routerStore";
  import { connectionState } from "../../stores/appStore";
  import { authState, logout } from "../../stores/authStore";
  import type { Route } from "../../stores/routerStore";

  const PUBLIC_LINKS: { route: Route; label: string }[] = [
    { route: "/", label: "Markets" },
    { route: "/login", label: "Login" },
  ];

  const AUTH_LINKS: { route: Route; label: string }[] = [
    { route: "/", label: "Markets" },
    { route: "/trade", label: "Trade" },
    { route: "/wallet", label: "Wallet" },
    { route: "/zk-verify", label: "ZK Verify" },
  ];
</script>

<nav class="terminal-panel px-4 py-3 sm:px-6">
  <div class="flex items-center justify-between gap-4">
    <!-- Brand -->
    <div class="flex items-center gap-3 shrink-0">
      <p class="mono text-xs tracking-[0.18em] text-sky-300/80 hidden sm:block">MATCHING ENGINE</p>
    </div>

    <!-- Navigation Links -->
    {#if $authState.userId}
      <div class="flex items-center gap-1 rounded-lg bg-slate-900/60 p-1 border border-slate-800/60">
        {#each AUTH_LINKS as link}
          <a
            href="#{link.route}"
            class="px-3 py-1.5 rounded-md text-xs font-medium tracking-wide transition-all
              {$router === link.route
                ? 'bg-sky-500/15 text-sky-300 border border-sky-400/30'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50 border border-transparent'}"
          >
            {link.label}
          </a>
        {/each}
      </div>
    {:else}
      <div class="flex items-center gap-1 rounded-lg bg-slate-900/60 p-1 border border-slate-800/60">
        {#each PUBLIC_LINKS as link}
          <a
            href="#{link.route}"
            class="px-3 py-1.5 rounded-md text-xs font-medium tracking-wide transition-all
              {$router === link.route
                ? 'bg-sky-500/15 text-sky-300 border border-sky-400/30'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50 border border-transparent'}"
          >
            {link.label}
          </a>
        {/each}
      </div>
    {/if}

    <!-- User + Status -->
    <div class="flex items-center gap-3 shrink-0">
      {#if $authState.userId}
        <div class="mono rounded-lg border border-slate-700/80 bg-slate-900/80 px-2.5 py-1.5 text-xs text-slate-100">
          {$authState.username}
        </div>

        <button
          type="button"
          class="rounded-lg border border-slate-700/80 bg-slate-900/80 px-2.5 py-1.5 text-xs text-slate-100 transition hover:border-slate-500"
          onclick={() => {
            logout();
            router.navigate("/login");
          }}
        >
          Logout
        </button>
      {/if}

      <div class="flex items-center gap-1.5" title="WebSocket: {$connectionState.ws}">
        <span
          class="inline-block h-2 w-2 rounded-full {$connectionState.ws === 'connected' ? 'bg-emerald-400 shadow-[0_0_6px_rgba(16,185,129,0.6)]' : 'bg-rose-400 shadow-[0_0_6px_rgba(244,63,94,0.6)]'}"
        ></span>
        <span class="mono text-[10px] uppercase tracking-wider {$connectionState.ws === 'connected' ? 'text-emerald-400' : 'text-rose-400'}">
          {$connectionState.ws}
        </span>
      </div>
    </div>
  </div>
</nav>
