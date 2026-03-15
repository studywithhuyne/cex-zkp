import { writable } from "svelte/store";

export const selectedUserId = writable(1);

export const connectionState = writable({
  ws: "idle",
  rest: "ready",
});
