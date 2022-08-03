import { api } from "@/api";
import type { LimitPaginatedResult } from "@/types";
import type { Artifact, ArtifactVersion } from "@/types/artifacts";
import { toNameVersion } from "@/utils/versioning";
import { defineStore } from "pinia";

export const useArtifactsStore = defineStore("artifacts", {
  state: () => ({
    artifacts: [] as Artifact[],
    artifactsByName: {} as Record<string, Artifact>,
    cachedVersions: {} as Record<string, ArtifactVersion>,
  }),
  getters: {
    artifact(): (name: string) => Artifact | undefined {
      return (name: string) => this.artifactsByName[name];
    },
    models(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "model");
    },
    datasets(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "dataset");
    },
    isHead(): (version: ArtifactVersion) => boolean | undefined {
      return (version: ArtifactVersion) => this.artifact(version.artifact)?.latest_version?.id == version.id;
    },
  },
  actions: {
    async hydrate() {
      this.artifacts = (await api.get<Artifact[]>("/artifacts")).data;
      this.artifacts.forEach((artifact) => (this.artifactsByName[artifact.name] = artifact));
    },
    async dehydrate() {
      this.$reset();
    },
    clearVersionCache(artifact?: string, version?: string) {
      if (artifact == null) {
        // if no artifact set, clear everything
        this.cachedVersions = {};
      } else {
        if (version == null) {
          // if no version set, clear all versions for artifact
          const staleKeys = Object.keys(this.cachedVersions).filter((key) => key.split("@")[0] == artifact);
          staleKeys.forEach((key) => delete this.cachedVersions[key]);
        } else {
          // if artifact and version set, clear only that specific version
          const key = `${artifact}@${version}`;
          if (key in this.cachedVersions) {
            delete this.cachedVersions[key];
          }
        }
      }
    },
    cacheVersion(artifactVersion: ArtifactVersion) {
      const nameVersion = toNameVersion(artifactVersion);
      this.cachedVersions[nameVersion] = artifactVersion;
    },
    async getVersions(artifactId: string, branch = "main", limit = 10, after?: string) {
      return (
        await api.get<LimitPaginatedResult<ArtifactVersion>>(`/artifacts/${artifactId}/versions`, {
          params: { branch, limit, after },
        })
      ).data;
    },
    async getVersion(artifactId: string, version: string) {
      const artifact = `${artifactId}@${version}`;
      if (this.cachedVersions[artifact] != null) {
        return this.cachedVersions[artifact];
      }
      const artifactVersion = (await api.get<ArtifactVersion>(`/artifacts/${artifactId}/versions/${version}`)).data;
      this.cacheVersion(artifactVersion);
      return artifactVersion;
    },
    async getVersionByTag(artifactId: string, tag: string) {
      const artifactVersion = (await api.get<ArtifactVersion>(`/artifacts/${artifactId}/tags/${tag}`)).data;
      this.cacheVersion(artifactVersion);
      return artifactVersion;
    },
  },
});
