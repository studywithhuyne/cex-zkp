<script lang="ts">
  import { router } from "../../stores/routerStore";
  import { login, register } from "../../stores/authStore";

  let mode = $state<"login" | "register">("login");
  let username = $state("");
  let password = $state("");
  let isSubmitting = $state(false);
  let message = $state("");
  let isError = $state(false);

  async function submitAuth() {
    if (!username.trim() || !password.trim()) {
      message = "Username va password khong duoc de trong";
      isError = true;
      return;
    }

    isSubmitting = true;
    message = "";
    isError = false;

    try {
      if (mode === "login") {
        await login(username, password);
        message = "Dang nhap thanh cong";
      } else {
        await register(username, password);
        message = "Dang ky thanh cong";
      }
      router.navigate("/");
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "Xac thuc that bai";
      message = msg;
      isError = true;
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section class="mx-auto mt-8 max-w-md terminal-panel-strong p-6 sm:p-7">
  <div class="mb-5 flex items-center justify-between">
    <h1 class="text-lg font-semibold tracking-wide text-slate-100 uppercase">Account Access</h1>
    <span class="mono rounded bg-slate-800/70 px-2 py-1 text-[10px] text-slate-300">Dev Auth</span>
  </div>

  <div class="mb-5 grid grid-cols-2 rounded-lg border border-slate-800/70 bg-slate-900/60 p-1">
    <button
      type="button"
      class="rounded-md px-3 py-2 text-xs font-semibold tracking-wide transition {mode === 'login' ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800/70'}"
      onclick={() => (mode = "login")}
    >
      Login
    </button>
    <button
      type="button"
      class="rounded-md px-3 py-2 text-xs font-semibold tracking-wide transition {mode === 'register' ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800/70'}"
      onclick={() => (mode = "register")}
    >
      Register
    </button>
  </div>

  <form class="space-y-4" onsubmit={(e) => { e.preventDefault(); void submitAuth(); }}>
    <div>
      <label for="username" class="mb-1 block text-[11px] font-medium tracking-widest text-slate-400 uppercase">Username</label>
      <input
        id="username"
        type="text"
        bind:value={username}
        autocomplete="username"
        placeholder="alice"
        class="mono w-full rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none transition focus:border-sky-500/60"
      />
    </div>

    <div>
      <label for="password" class="mb-1 block text-[11px] font-medium tracking-widest text-slate-400 uppercase">Password</label>
      <input
        id="password"
        type="password"
        bind:value={password}
        autocomplete={mode === "login" ? "current-password" : "new-password"}
        placeholder="At least 8 characters"
        class="mono w-full rounded-lg border border-slate-700/80 bg-slate-900/80 px-3 py-2 text-sm text-slate-100 outline-none transition focus:border-sky-500/60"
      />
    </div>

    <button
      type="submit"
      disabled={isSubmitting}
      class="h-10 w-full rounded-lg border border-sky-500/40 bg-sky-500/20 text-sm font-semibold tracking-wider text-sky-200 transition hover:bg-sky-500/30 disabled:cursor-not-allowed disabled:opacity-60"
    >
      {isSubmitting ? "Processing..." : mode === "login" ? "Login" : "Create account"}
    </button>

    {#if message}
      <p class="mono rounded px-3 py-2 text-xs {isError ? 'bg-rose-500/10 text-rose-300' : 'bg-emerald-500/10 text-emerald-300'}">
        {message}
      </p>
    {/if}
  </form>
</section>
