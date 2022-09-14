import { defineStore } from "pinia";

export const useUserStore = defineStore("user", {
  state: () => ({
    currentOrganization: null as string | null,
  }),
});
