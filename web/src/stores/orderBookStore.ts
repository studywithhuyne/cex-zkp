import { writable } from "svelte/store";
import { connectionState } from "./appStore";

const WS_URL = `${import.meta.env.VITE_WS_URL || "ws://127.0.0.1:8080"}/ws`;

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

type SnapshotMessage = {
  type: "snapshot";
  bids?: PriceLevel[];
  asks?: PriceLevel[];
};

type UpdateMessage = {
  type: "update";
  bids?: PriceLevel[];
  asks?: PriceLevel[];
};

type TradeMessage = {
  type: "trade";
  trade: Trade;
};

type WsMessage = SnapshotMessage | UpdateMessage | TradeMessage;

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
        const data = JSON.parse(event.data) as WsMessage | { type?: string };
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

  function handleMessage(msg: WsMessage | { type?: string }) {
    update(state => {
      if (msg.type === "snapshot" && "bids" in msg && "asks" in msg) {
        return {
          ...state,
          bids: msg.bids || [],
          asks: msg.asks || [],
        };
      }

      if (msg.type === "update") {
        return {
          ...state,
          bids: "bids" in msg && Array.isArray(msg.bids) ? msg.bids : state.bids,
          asks: "asks" in msg && Array.isArray(msg.asks) ? msg.asks : state.asks,
        };
      }

      if (msg.type === "trade" && "trade" in msg) {
        return {
          ...state,
          trades: [msg.trade, ...state.trades].slice(0, 50),
        };
      }

      return state;
    });
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
