import { defineStore } from "pinia";

export const useUserStore = defineStore("user", {
  state: () => ({
    currentOrganization: "local" as string | null,
  }),
});
