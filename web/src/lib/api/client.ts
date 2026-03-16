const DEFAULT_HEADERS = {
  "Content-Type": "application/json",
};

type AuthUserId = string | number;

export async function apiGet<T = unknown>(path: string, userId: AuthUserId = 1): Promise<T> {
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
  userId: AuthUserId = 1,
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

export async function apiDelete(path: string, userId: AuthUserId = 1): Promise<void> {
  const response = await fetch(path, {
    method: "DELETE",
    headers: { "x-user-id": String(userId) },
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(err.error || `DELETE ${path} failed`);
  }
}

export async function apiPostPublic<T = unknown>(
  path: string,
  body: unknown,
): Promise<T> {
  const response = await fetch(path, {
    method: "POST",
    headers: DEFAULT_HEADERS,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(err.error || `POST ${path} failed`);
  }

  return response.json() as Promise<T>;
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

export type AuthResponse = {
  user_id: string;
  username: string;
  auth_mode: string;
  auth_header: string;
};

export type BalanceDto = {
  asset: string;
  available: string;
  locked: string;
};

export type AveragePriceDto = {
  symbol: string;
  best_bid: string | null;
  best_ask: string | null;
  mid_price: string | null;
  micro_price: string | null;
};

export type CandleDto = {
  time: number;
  open: string;
  high: string;
  low: string;
  close: string;
  volume: string;
};

export const fetchOpenOrders = (userId: AuthUserId) =>
  apiGet<OpenOrder[]>("/api/orders/open", userId);

export const fetchRecentTrades = () =>
  apiGet<RecentTrade[]>("/api/trades/recent");

export const fetchUserTrades = (userId: AuthUserId) =>
  apiGet<UserTrade[]>("/api/trades/user", userId);

export const fetchBalances = (userId: AuthUserId) =>
  apiGet<BalanceDto[]>("/api/balances", userId);

export const fetchBalanceByAsset = (userId: AuthUserId, asset: string) =>
  apiGet<BalanceDto>(`/api/balances/${encodeURIComponent(asset)}`, userId);

export const fetchAveragePrice = (symbol = "BTC_USDT") =>
  apiGet<AveragePriceDto>(`/api/price/average?symbol=${encodeURIComponent(symbol)}`);

export const fetchCandles = (symbol: string, interval = "1d", limit = 1) =>
  apiGet<CandleDto[]>(
    `/api/candles?symbol=${encodeURIComponent(symbol)}&interval=${encodeURIComponent(interval)}&limit=${limit}`,
  );

export const postDeposit = (userId: AuthUserId, asset: string, amount: string) =>
  apiPost<DepositResponse>("/api/deposit", { asset, amount }, userId);

export const cancelOrder = (userId: AuthUserId, orderId: number) =>
  apiDelete(`/api/orders/${orderId}`, userId);

export const postLogin = (username: string, password: string) =>
  apiPostPublic<AuthResponse>("/api/auth/login", { username, password });

export const postRegister = (username: string, password: string) =>
  apiPostPublic<AuthResponse>("/api/auth/register", { username, password });

export const fetchAuthMe = (userId: AuthUserId) =>
  apiGet<AuthResponse>("/api/auth/me", userId);
