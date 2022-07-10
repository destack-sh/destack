import { api } from "@/api";
import type { ModelHandlerSpec } from "@/types";
import { defineStore } from "pinia";

export const useMetaStore = defineStore("meta", {
  state: () => ({
    modelHandlers: [] as Array<ModelHandlerSpec>,
  }),
  getters: {},
  actions: {
    async hydrate() {
      this.modelHandlers = (await api.get<ModelHandlerSpec[]>("/meta/models")).data;
    },
    async dehyrate() {
      this.$reset();
    },
  },
});
