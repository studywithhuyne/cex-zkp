import { derived, writable } from "svelte/store";
import { fetchAuthMe, postLogin, postRegister } from "../lib/api/client";

type AuthStoreState = {
  userId: string | null;
  username: string | null;
  loading: boolean;
};

const AUTH_STORAGE_KEY = "cex_auth_v1";

const INITIAL_STATE: AuthStoreState = {
  userId: null,
  username: null,
  loading: true,
};

const store = writable<AuthStoreState>(INITIAL_STATE);

function setAuthenticated(userId: string, username: string) {
  localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify({ userId, username }));
  store.set({ userId, username, loading: false });
}

function clearAuth() {
  localStorage.removeItem(AUTH_STORAGE_KEY);
  store.set({ userId: null, username: null, loading: false });
}

export const authState = {
  subscribe: store.subscribe,
};

export const isAuthenticated = derived(authState, ($authState) => $authState.userId !== null);

export async function bootstrapAuth(): Promise<void> {
  try {
    const raw = localStorage.getItem(AUTH_STORAGE_KEY);
    if (!raw) {
      store.update((s) => ({ ...s, loading: false }));
      return;
    }

    const parsed = JSON.parse(raw) as { userId?: unknown; username?: unknown };
    if (typeof parsed.userId !== "string" || !/^\d+$/.test(parsed.userId) || parsed.userId === "0") {
      clearAuth();
      return;
    }

    const me = await fetchAuthMe(parsed.userId);
    setAuthenticated(me.user_id, me.username);
  } catch {
    clearAuth();
  }
}

export async function login(username: string, password: string): Promise<void> {
  const res = await postLogin(username, password);
  setAuthenticated(res.user_id, res.username);
}

export async function register(username: string, password: string): Promise<void> {
  const res = await postRegister(username, password);
  setAuthenticated(res.user_id, res.username);
}

export function logout(): void {
  clearAuth();
}
