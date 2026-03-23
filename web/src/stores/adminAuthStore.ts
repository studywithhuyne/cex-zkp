import { writable } from "svelte/store";

export const adminAuthState = writable({
  isLoggedIn: false,
});

export const loginAdmin = (username?: string, password?: string) => {
  if (username === "admin" && password === "admin") {
    adminAuthState.set({ isLoggedIn: true });
    localStorage.setItem("admin_token", "true");
    return true;
  }
  return false;
};

export const logoutAdmin = () => {
  adminAuthState.set({ isLoggedIn: false });
  localStorage.removeItem("admin_token");
};

export const bootstrapAdminAuth = () => {
  if (localStorage.getItem("admin_token") === "true") {
    adminAuthState.set({ isLoggedIn: true });
  }
};

