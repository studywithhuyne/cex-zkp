<script lang="ts">
  import { router } from "../../stores/routerStore";
  import { selectedUserId, connectionState, MOCK_USERS } from "../../stores/appStore";
  import type { Route } from "../../stores/routerStore";

  const NAV_LINKS: { route: Route; label: string }[] = [
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
    <div class="flex items-center gap-1 rounded-lg bg-slate-900/60 p-1 border border-slate-800/60">
      {#each NAV_LINKS as link}
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

    <!-- User Selector + Status -->
    <div class="flex items-center gap-3 shrink-0">
      <select
        bind:value={$selectedUserId}
        class="rounded-lg border border-slate-700/80 bg-slate-900/80 px-2.5 py-1.5 text-xs text-slate-100 outline-none transition focus:border-sky-500/50 cursor-pointer"
      >
        {#each MOCK_USERS as user}
          <option value={user.id}>{user.name} (ID: {user.id})</option>
        {/each}
      </select>

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
