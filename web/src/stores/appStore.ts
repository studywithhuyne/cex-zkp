import { writable } from "svelte/store";

export const MOCK_USERS = [
  { id: "1", name: "Alice" },
  { id: "2", name: "Bob" },
  { id: "3", name: "Charlie" },
  { id: "4", name: "Dave" },
] as const;

export const selectedUserId = writable("1");

export const connectionState = writable({
  ws: "idle",
  rest: "ready",
});

/** Increment to trigger balance refetch across components. */
export const balanceVersion = writable(0);
