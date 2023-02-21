import { defineStore } from "pinia";
import { toRef } from "vue";

export const NON_SOCIAL_AUTH_ENABLED = process.env.ENVIRONMENT === "development";

export const useAuthStore = defineStore("auth", {
  state: () => ({
    token: null as string | null,
    loggedIn: false,
  }),
  actions: {
    setLoggedIn(token: string): void {
      this.token = token;
      this.loggedIn = true;
    },
    setLoggedOut(): void {
      this.token = null;
      this.loggedIn = false;
    },
  },
});

export function useAuth() {
  const state = useAuthStore();

  return { loggedIn: toRef(state, "loggedIn") };
}
