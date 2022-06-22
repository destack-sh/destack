import { api } from "@/api";
import type { Artifact } from "@/types/artifacts";
import { defineStore } from "pinia";

export const useArtifactsStore = defineStore("artifacts", {
  state: () => ({
    artifacts: [] as Artifact[],
  }),
  actions: {
    async hydrate() {
      this.$state.artifacts = (await api.get<Artifact[]>("/artifacts")).data;
    },
    async dehydrate() {
      this.$reset();
    },
  },
});
