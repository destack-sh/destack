import { defineStore } from "pinia";

export const useAppStore = defineStore("app", {
  state: () => ({
    error: null as Error | null,
    hydrated: false,
    hydrating: false,
    authenticated: false,
  }),
});
