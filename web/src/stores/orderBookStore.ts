import { writable } from "svelte/store";
import { connectionState } from "./appStore";

// WebSocket URL: dynamic so it works in both dev (Vite proxy on :5173) and Docker (Nginx on :8080).
// Override via VITE_WS_URL env var if needed.
const WS_URL = import.meta.env.VITE_WS_URL ?? `ws://${location.host}/ws`;

type WsStatus = "idle" | "connecting" | "connected" | "disconnected";

type PriceLevel = {
  price: number;
  amount: number;
};

type Trade = {
  price: number;
  amount: number;
  side?: "buy" | "sell";
  ts?: number;
};

type OrderBookState = {
  bids: PriceLevel[];
  asks: PriceLevel[];
  trades: Trade[];
};

// ── Backend WsEvent format (matches core/src/api/ws.rs) ──────────────────────
// {"type": "orderbook_update", "data": {"bids": [{"price":"..","amount":".."}], "asks": [...]}}
// {"type": "trade_executed",   "data": {"price": "..", "amount": ".."}}
type WsApiPriceLevel = { price: string; amount: string };

type WsOrderbookUpdate = {
  type: "orderbook_update";
  data: { bids: WsApiPriceLevel[]; asks: WsApiPriceLevel[] };
};

type WsTradeExecuted = {
  type: "trade_executed";
  data: { price: string; amount: string };
};

type WsApiMessage = WsOrderbookUpdate | WsTradeExecuted;

const EMPTY_STATE: OrderBookState = {
  bids: [],
  asks: [],
  trades: [],
};

function createOrderBookStore() {
  const { subscribe, set, update } = writable<OrderBookState>(EMPTY_STATE);

  let socket: WebSocket | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function connect() {
    if (socket && (socket.readyState === WebSocket.CONNECTING || socket.readyState === WebSocket.OPEN)) {
      return;
    }

    connectionState.update(s => ({ ...s, ws: "connecting" as WsStatus }));
    socket = new WebSocket(WS_URL);

    socket.onopen = () => {
      connectionState.update(s => ({ ...s, ws: "connected" as WsStatus }));
      console.log("WebSocket connected to", WS_URL);
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as WsApiMessage;
        handleMessage(data);
      } catch (err) {
        console.error("Failed to parse WS message", err);
      }
    };

    socket.onclose = () => {
      connectionState.update(s => ({ ...s, ws: "disconnected" as WsStatus }));
      console.log("WebSocket disconnected. Reconnecting in 3s...");
      scheduleReconnect();
    };

    socket.onerror = (error) => {
      console.error("WebSocket error:", error);
      socket?.close();
    };
  }

  function scheduleReconnect() {
    if (!reconnectTimer) {
      reconnectTimer = setTimeout(() => {
        connect();
      }, 3000);
    }
  }

  function handleMessage(msg: WsApiMessage) {
    // orderbook_update: full depth snapshot after any book mutation
    if (msg.type === "orderbook_update") {
      update(state => ({
        ...state,
        bids: msg.data.bids.map(b => ({ price: parseFloat(b.price), amount: parseFloat(b.amount) })),
        asks: msg.data.asks.map(a => ({ price: parseFloat(a.price), amount: parseFloat(a.amount) })),
      }));
      return;
    }

    // trade_executed: single fill event
    if (msg.type === "trade_executed") {
      const trade: Trade = {
        price:  parseFloat(msg.data.price),
        amount: parseFloat(msg.data.amount),
        ts:     Date.now(),
      };
      update(state => ({
        ...state,
        trades: [trade, ...state.trades].slice(0, 50),
      }));
      return;
    }
  }

  function disconnect() {
    if (socket) {
      socket.close();
      socket = null;
    }
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    set(EMPTY_STATE);
  }

  return {
    subscribe,
    connect,
    disconnect,
  };
}

export const orderBook = createOrderBookStore();
