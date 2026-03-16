const DEFAULT_HEADERS = {
  "Content-Type": "application/json",
};

export async function apiGet<T = unknown>(path: string, userId = 1): Promise<T> {
  const response = await fetch(path, {
    method: "GET",
    headers: {
      ...DEFAULT_HEADERS,
      "x-user-id": String(userId),
    },
  });

  if (!response.ok) {
    throw new Error(`GET ${path} failed with status ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export async function apiPost<T = unknown>(
  path: string,
  body: unknown,
  userId = 1,
): Promise<T> {
  const response = await fetch(path, {
    method: "POST",
    headers: {
      ...DEFAULT_HEADERS,
      "x-user-id": String(userId),
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(err.error || `POST ${path} failed`);
  }

  return response.json() as Promise<T>;
}

export async function apiDelete(path: string, userId = 1): Promise<void> {
  const response = await fetch(path, {
    method: "DELETE",
    headers: { "x-user-id": String(userId) },
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(err.error || `DELETE ${path} failed`);
  }
}

// ── Typed endpoint helpers ──────────────────────────────────────────────────

export type OpenOrder = {
  order_id: number;
  side: string;
  price: string;
  amount: string;
  filled: string;
  status: string;
  base_asset: string;
  quote_asset: string;
  created_at: string;
};

export type RecentTrade = {
  price: string;
  amount: string;
  base_asset: string;
  quote_asset: string;
  executed_at: string;
};

export type UserTrade = {
  id: string;
  maker_order_id: number;
  taker_order_id: number;
  side: string;
  price: string;
  amount: string;
  base_asset: string;
  quote_asset: string;
  executed_at: string;
};

export type DepositResponse = {
  asset: string;
  deposited: string;
  new_available: string;
};

export type BalanceDto = {
  asset: string;
  available: string;
  locked: string;
};

export const fetchOpenOrders = (userId: number) =>
  apiGet<OpenOrder[]>("/api/orders/open", userId);

export const fetchRecentTrades = () =>
  apiGet<RecentTrade[]>("/api/trades/recent");

export const fetchUserTrades = (userId: number) =>
  apiGet<UserTrade[]>("/api/trades/user", userId);

export const fetchBalances = (userId: number) =>
  apiGet<BalanceDto[]>("/api/balances", userId);

export const postDeposit = (userId: number, asset: string, amount: string) =>
  apiPost<DepositResponse>("/api/deposit", { asset, amount }, userId);

export const cancelOrder = (userId: number, orderId: number) =>
  apiDelete(`/api/orders/${orderId}`, userId);
