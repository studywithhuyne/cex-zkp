import { writable } from "svelte/store";

export type Route = "/trade" | "/wallet" | "/zk-verify";

const VALID_ROUTES: Route[] = ["/trade", "/wallet", "/zk-verify"];
const DEFAULT_ROUTE: Route = "/trade";

function parseHash(): Route {
  const raw = window.location.hash.replace(/^#/, "") || "";
  return VALID_ROUTES.includes(raw as Route) ? (raw as Route) : DEFAULT_ROUTE;
}

function createRouter() {
  const { subscribe, set } = writable<Route>(parseHash());

  window.addEventListener("hashchange", () => set(parseHash()));

  function navigate(route: Route) {
    window.location.hash = route;
  }

  return { subscribe, navigate };
}

export const router = createRouter();
