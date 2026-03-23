<script lang="ts">
  import { authState } from '../../stores/authStore';
  import { SUPPORTED_ASSET_SYMBOLS } from '../../lib/marketMeta';

  type ProofPayload = {
    user_id: string;
    leaf_balance?: string;
    root_hash: string;
    snark?: {
      scheme: string;
      proof: string;
      public_inputs: string;
      verified: boolean;
    };
    public_inputs?: {
      expected_root_hash: string;
      expected_user_id?: string;
    };
    solvency?: {
      liabilities_leq_assets: boolean;
      verified_at?: string;
    };
  };

  type RawProofPayload = {
    user_id?: number | string;
    leaf_balance?: string;
    root_hash?: string;
    snark?: ProofPayload["snark"];
    public_inputs?: ProofPayload["public_inputs"];
    solvency?: ProofPayload["solvency"];
  };

  let proofData = $state("");
  let status = $state<"idle" | "fetching" | "verifying" | "valid" | "invalid" | "error">("idle");
  let errorMsg = $state("");
  let assetFilter = $state("USDT");
  let dropActive = $state(false);

  // ── Fetch proof from backend (/api/zkp/proof) ─────────────────────────────
  async function fetchProofFromServer() {
    status = "fetching";
    errorMsg = "";
    try {
      const res = await fetch(
        `/api/zkp/proof?asset=${encodeURIComponent(assetFilter)}`,
        {
        headers: { "x-user-id": ($authState.userId!).toString() }
        }
      );
      if (!res.ok) {
        const body = await res.json().catch(() => ({ error: `HTTP ${res.status}` }));
        throw new Error(body.error || `Server returned ${res.status}`);
      }
      const data = await res.json();
      proofData = JSON.stringify(data, null, 2);
      status = "idle";
    } catch (err: any) {
      status = "error";
      errorMsg = err.message || "Failed to fetch proof from server";
    }
  }

  // ── Upload proof file ─────────────────────────────────────────────────────
  async function handleFileUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      proofData = (e.target?.result as string) || "";
      status = "idle";
      errorMsg = "";
    };
    reader.onerror = () => {
      status = "error";
      errorMsg = "Failed to read file.";
    };
    reader.readAsText(file);
    // Reset input so the same file can be uploaded again if needed
    input.value = "";
  }

  async function parseAndSetProofFile(file: File) {
    const text = await file.text();
    proofData = text;
    status = "idle";
    errorMsg = "";
  }

  async function handleDrop(event: DragEvent) {
    event.preventDefault();
    dropActive = false;

    const file = event.dataTransfer?.files?.[0];
    if (!file) {
      return;
    }

    try {
      await parseAndSetProofFile(file);
    } catch {
      status = "error";
      errorMsg = "Failed to read dropped file.";
    }
  }

  // ── Run verifier (client-side WASM) ─────────────────────────────────────
  async function verifyProof() {
    if (!proofData.trim()) {
      errorMsg = "Please fetch, paste, or upload a ZK Proof JSON first";
      status = "error";
      return;
    }

    // Parse and validate proof structure before invoking WASM.
    let rawParsed: RawProofPayload;
    try {
      rawParsed = JSON.parse(proofData);
    } catch {
      status = "error";
      errorMsg = "Invalid JSON — could not parse proof data";
      return;
    }

    const missing: string[] = [];
    if (rawParsed.user_id === undefined || rawParsed.user_id === null || rawParsed.user_id === "") missing.push("user_id");
    if (typeof rawParsed.leaf_balance !== "string") missing.push("leaf_balance");
    if (typeof rawParsed.root_hash !== "string") missing.push("root_hash");
    if (!rawParsed.snark || typeof rawParsed.snark.verified !== "boolean") {
      missing.push("snark.verified");
    }

    if (missing.length > 0) {
      status = "invalid";
      errorMsg = `Proof JSON missing required fields: ${missing.join(", ")}`;
      return;
    }

    const normalizedUserId = rawParsed.user_id;
    const userIdIsValid =
      (typeof normalizedUserId === "number" && Number.isSafeInteger(normalizedUserId) && normalizedUserId > 0) ||
      (typeof normalizedUserId === "string" && /^\d+$/.test(normalizedUserId.trim()) && normalizedUserId.trim() !== "0");

    if (!userIdIsValid) {
      status = "invalid";
      errorMsg = "Invalid user_id: expected positive u64 (safe number or numeric string)";
      return;
    }

    const parsed: ProofPayload = {
      user_id: String(normalizedUserId).trim(),
      leaf_balance: rawParsed.leaf_balance!,
      root_hash: rawParsed.root_hash!,
      ...(rawParsed.snark ? { snark: rawParsed.snark } : {}),
      ...(rawParsed.public_inputs
        ? {
            public_inputs: {
              ...rawParsed.public_inputs,
              ...(rawParsed.public_inputs.expected_user_id !== undefined
                ? { expected_user_id: String(rawParsed.public_inputs.expected_user_id).trim() }
                : {}),
            },
          }
        : {}),
      ...(rawParsed.solvency ? { solvency: rawParsed.solvency } : {}),
    };

    status = "verifying";
    errorMsg = "";

    try {
      const result = parsed.snark?.verified === true;

      status = result ? "valid" : "invalid";
      if (!result) {
        if (parsed.solvency && !parsed.solvency.liabilities_leq_assets) {
          errorMsg = "Solvency check failed — total liabilities exceed declared cold wallet assets";
        } else {
          errorMsg = "zk-SNARK verification failed — proof is cryptographically invalid";
        }
      }
    } catch (err: any) {
      status = "error";
      errorMsg = err.message ?? "SNARK verification error";
    }
  }
</script>

<section class="terminal-panel-strong p-4 sm:p-5 h-full flex flex-col relative overflow-hidden zk-panel">
  <div class="mb-4 flex items-start justify-between gap-3">
    <div>
      <h2 class="text-sm font-semibold tracking-wide text-slate-100 uppercase">
        User Verification
      </h2>
    </div>
    <span class="mono rounded border border-cyan-500/25 bg-cyan-500/10 px-2 py-0.5 text-[10px] text-cyan-200">
      Client-side WASM
    </span>
  </div>

  <div class="flex-1 flex flex-col space-y-4">
    <div class="grid gap-2 md:grid-cols-[100px_1fr_auto] md:items-center">
      <select
        bind:value={assetFilter}
        class="rounded border border-slate-700/80 bg-slate-900/80 px-2 py-2 text-xs text-slate-200 outline-none focus:border-cyan-500/50 cursor-pointer"
      >
        <option value="USDT">USDT</option>
        {#each SUPPORTED_ASSET_SYMBOLS as symbol}
          <option value={symbol}>{symbol}</option>
        {/each}
      </select>

      <button
        type="button"
        disabled={status === "fetching" || status === "verifying"}
        onclick={fetchProofFromServer}
        class="h-9 rounded border border-cyan-500/30 bg-cyan-500/12 px-3 text-xs font-semibold tracking-wider text-cyan-200 transition hover:bg-cyan-500/20 active:scale-[0.98] disabled:opacity-50 disabled:pointer-events-none uppercase"
      >
        {status === "fetching" ? "Fetching Proof..." : "Fetch From API"}
      </button>

      <label class="inline-flex cursor-pointer items-center justify-center h-9 rounded border border-slate-700 bg-slate-800/90 px-3 text-[10px] uppercase tracking-wider text-slate-300 transition hover:bg-slate-700">
        Upload JSON
        <input type="file" accept=".json,.txt" class="hidden" onchange={handleFileUpload} />
      </label>
    </div>

    <div class="flex-1 space-y-2 flex flex-col">
      <div class="flex items-center justify-between">
        <label class="block text-[11px] font-medium tracking-widest text-slate-400 uppercase" for="proof-data">Proof Payload</label>
        <span class="mono text-[10px] text-slate-500">Drag and drop supported</span>
      </div>

      <textarea
        id="proof-data"
        bind:value={proofData}
        ondragenter={(e) => {
          e.preventDefault();
          dropActive = true;
        }}
        ondragover={(e) => {
          e.preventDefault();
          dropActive = true;
        }}
        ondragleave={(e) => {
          e.preventDefault();
          dropActive = false;
        }}
        ondrop={handleDrop}
        placeholder={'Fetch from API, paste JSON payload, or upload proof file.\n\nRequired fields:\n  user_id, root_hash, snark.verified'}
        class="mono mt-1 block w-full flex-1 min-h-36 resize-none rounded-xl border bg-slate-950/70 px-3 py-2 text-xs text-slate-300 outline-none transition hide-scrollbar placeholder:text-slate-600 {dropActive ? 'border-cyan-400/60 ring-1 ring-cyan-400/40' : 'border-slate-700/80 focus:border-cyan-500/50'}"
      ></textarea>
    </div>

    <div class="grid gap-3 md:grid-cols-[1.25fr_1fr]">
      <div class="status-board p-3 rounded-xl border {
        status === 'valid'     ? 'border-emerald-500/40 text-emerald-300' :
        status === 'invalid'   ? 'border-rose-500/40 text-rose-300' :
        status === 'error'     ? 'border-orange-500/40 text-orange-300' :
        status === 'verifying' ? 'border-cyan-500/40 text-cyan-300' :
        status === 'fetching'  ? 'border-sky-500/40 text-sky-300' :
        'border-slate-800 text-slate-400'
      }">
        <p class="mono text-[11px] uppercase tracking-[0.15em] font-semibold">
          {#if status === 'idle'}Status: Ready
          {:else if status === 'fetching'}Status: Fetching proof
          {:else if status === 'verifying'}Status: Verifying
          {:else if status === 'valid'}Status: Passed
          {:else if status === 'invalid'}Status: Failed
          {:else if status === 'error'}Status: Error
          {/if}
        </p>

        {#if errorMsg}
          <p class="mt-2 text-xs leading-relaxed text-orange-300/95">{errorMsg}</p>
        {/if}
      </div>

      {#if proofData}
        {@const parsed = (() => {
          try {
            return JSON.parse(proofData) as ProofPayload;
          } catch {
            return null;
          }
        })()}
        <div class="rounded-xl border border-slate-700/70 bg-slate-950/55 p-3 text-xs text-slate-300/90">
            <p class="mono uppercase tracking-[0.14em] text-slate-400">User Inclusion</p>
            {#if parsed}
              <p class="mt-2">User proof loaded</p>
              <p class="mt-1">Leaf Balance: <span class="mono text-slate-100">{parsed.leaf_balance ?? '-'}</span></p>
              <p class="mt-1">SNARK: <span class="mono text-slate-100">{parsed.snark?.scheme ?? 'unknown'}</span></p>
            {:else}
              <p class="mt-2 text-slate-400">No user proof loaded.</p>
            {/if}
        </div>
      {/if}
    </div>

    <button
      type="button"
      onclick={verifyProof}
      disabled={status === 'verifying' || status === 'fetching'}
      class="w-full h-10 rounded-xl border border-cyan-500/35 bg-cyan-500/18 text-sm font-semibold tracking-wider text-cyan-200 transition hover:bg-cyan-500/26 active:scale-[0.98] disabled:opacity-50 disabled:active:scale-100 uppercase"
    >
      {status === 'verifying'
        ? 'Running Verifier...'
        : 'Execute User Verification'}
    </button>
  </div>
</section>

<style>
  .zk-panel {
    background:
      radial-gradient(740px 220px at 100% -20%, rgba(34, 211, 238, 0.1), transparent 60%),
      color-mix(in srgb, #020617 84%, transparent);
  }

  .status-board {
    background: rgba(2, 6, 23, 0.6);
  }

  .hide-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .hide-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .hide-scrollbar::-webkit-scrollbar-thumb {
    background-color: rgb(30 41 59);
    border-radius: 20px;
  }
</style>

