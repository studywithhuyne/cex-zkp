import { writable } from "svelte/store";

export const markets = writable<string[]>(["BTC_USDT"]);
export const selectedMarket = writable<string>("BTC_USDT");
