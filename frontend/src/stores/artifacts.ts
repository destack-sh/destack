import { api } from "@/api";
import type { Artifact, ArtifactVersion } from "@/types/artifacts";
import { defineStore } from "pinia";

export const useArtifactsStore = defineStore("artifacts", {
  state: () => ({
    artifacts: [] as Artifact[],
    versions: new Map<string, ArtifactVersion[]>(),
  }),
  getters: {
    models(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "model");
    },
    datasets(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "dataset");
    },
  },
  actions: {
    async hydrate() {
      this.$state.artifacts = (await api.get<Artifact[]>("/artifacts")).data;
    },
    async dehydrate() {
      this.$reset();
    },
  },
});
