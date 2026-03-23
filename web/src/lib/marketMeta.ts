export type MarketAsset = {
  symbol: "BTC" | "ETH" | "SOL" | "BNB";
  pair: string;
  iconUrl: string;
};

export const SUPPORTED_MARKET_ASSETS: MarketAsset[] = [
  {
    symbol: "BTC",
    pair: "BTC_USDT",
    iconUrl: "/icons/coins/btc.svg",
  },
  {
    symbol: "ETH",
    pair: "ETH_USDT",
    iconUrl: "/icons/coins/eth.svg",
  },
  {
    symbol: "SOL",
    pair: "SOL_USDT",
    iconUrl: "/icons/coins/sol.svg",
  },
  {
    symbol: "BNB",
    pair: "BNB_USDT",
    iconUrl: "/icons/coins/bnb.svg",
  },
];

export const SUPPORTED_PAIRS = SUPPORTED_MARKET_ASSETS.map((asset) => asset.pair);
export const SUPPORTED_ASSET_SYMBOLS = SUPPORTED_MARKET_ASSETS.map((asset) => asset.symbol);
