import { api } from "@/api";
import type { LimitPaginatedResult } from "@/types";
import type { Artifact, ArtifactVersion } from "@/types/artifacts";
import { defineStore } from "pinia";

export const useArtifactsStore = defineStore("artifacts", {
  state: () => ({
    artifacts: [] as Artifact[],
    artifactsByName: new Map<string, Artifact>(),
    versions: new Map<string, ArtifactVersion[]>(),
  }),
  getters: {
    artifact() {
      return (name: string) => this.artifactsByName.get(name);
    },
    models(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "model");
    },
    datasets(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "dataset");
    },
    isHead(): (version: ArtifactVersion) => boolean | undefined {
      return (version: ArtifactVersion) =>
        this.artifact(version.artifact)?.latest_version?.id == version.id;
    },
  },
  actions: {
    async hydrate() {
      this.artifacts = (await api.get<Artifact[]>("/artifacts")).data;
      this.artifacts.forEach((artifact) => this.artifactsByName.set(artifact.name, artifact));
    },
    async dehydrate() {
      this.$reset();
    },
    async getVersions(artifactId: string, branch = "main", limit = 10, after?: string) {
      return (
        await api.get<LimitPaginatedResult<ArtifactVersion>>(`/artifacts/${artifactId}/versions`, {
          params: { branch, limit, after },
        })
      ).data;
    },
    async getVersion(artifactId: string, version: string) {
      return (await api.get<ArtifactVersion>(`/artifacts/${artifactId}/versions/${version}`)).data;
    },
    async getVersionByTag(artifactId: string, tag: string) {
      return (await api.get<ArtifactVersion>(`/artifacts/${artifactId}/tags/${tag}`)).data;
    },
  },
});
