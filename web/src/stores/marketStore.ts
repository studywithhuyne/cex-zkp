import { writable } from "svelte/store";
import { SUPPORTED_PAIRS } from "../lib/marketMeta";

const preferred =
	typeof localStorage !== "undefined"
		? localStorage.getItem("preferred_trade_symbol") ?? ""
		: "";

const initialMarket = SUPPORTED_PAIRS.includes(preferred) ? preferred : "BTC_USDT";

export const markets = writable<string[]>(SUPPORTED_PAIRS);
export const selectedMarket = writable<string>(initialMarket);
