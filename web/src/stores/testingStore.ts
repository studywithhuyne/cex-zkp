import { writable } from "svelte/store";

// Centralized store for testing parameters like the mocked exchange cold wallet assets.
export const testingConfig = {
  coldWalletAssets: writable("500000000")
};
