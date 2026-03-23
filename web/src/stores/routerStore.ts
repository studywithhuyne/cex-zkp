import { writable } from "svelte/store";

export type Route = "/" | "/login" | "/trade" | "/asset" | "/trade-history" | "/zk-verify" | "/testing" | "/admin" | "/admin/login";

const VALID_ROUTES: Route[] = ["/", "/login", "/trade", "/asset", "/trade-history", "/zk-verify", "/testing", "/admin", "/admin/login"];
const DEFAULT_ROUTE: Route = "/";

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
