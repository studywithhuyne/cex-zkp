import { writable } from "svelte/store";
import { connectionState } from "./appStore";

const WS_URL = `${import.meta.env.VITE_WS_URL || "ws://127.0.0.1:8080"}/ws`;

function createOrderBookStore() {
  const { subscribe, set, update } = writable({
    bids: [],
    asks: [],
    trades: [],
  });

  let socket = null;
  let reconnectTimer = null;

  function connect() {
    if (socket && (socket.readyState === WebSocket.CONNECTING || socket.readyState === WebSocket.OPEN)) {
      return;
    }

    connectionState.update(s => ({ ...s, ws: "connecting" }));
    socket = new WebSocket(WS_URL);

    socket.onopen = () => {
      connectionState.update(s => ({ ...s, ws: "connected" }));
      console.log("WebSocket connected to", WS_URL);
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        handleMessage(data);
      } catch (err) {
        console.error("Failed to parse WS message", err);
      }
    };

    socket.onclose = () => {
      connectionState.update(s => ({ ...s, ws: "disconnected" }));
      console.log("WebSocket disconnected. Reconnecting in 3s...");
      scheduleReconnect();
    };

    socket.onerror = (error) => {
      console.error("WebSocket error:", error);
      socket.close();
    };
  }

  function scheduleReconnect() {
    if (!reconnectTimer) {
      reconnectTimer = setTimeout(() => {
        connect();
      }, 3000);
    }
  }

  function handleMessage(msg) {
    update(state => {
      // Mocked up logic expecting 'snapshot', 'update', or 'trade' types from UI-02 specs
      if (msg.type === "snapshot") {
        return {
          ...state,
          bids: msg.bids || [],
          asks: msg.asks || [],
        };
      } else if (msg.type === "update") {
        // Here we'd apply partial orderbook diffs
        // For simplicity now if backend sends full sides:
        if (msg.bids) state.bids = msg.bids;
        if (msg.asks) state.asks = msg.asks;
      } else if (msg.type === "trade") {
        state.trades = [msg.trade, ...state.trades].slice(0, 50); // Keep last 50
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
    set({ bids: [], asks: [], trades: [] });
  }

  return {
    subscribe,
    connect,
    disconnect,
  };
}

export const orderBook = createOrderBookStore();
