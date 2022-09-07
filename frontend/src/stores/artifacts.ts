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
      return (name) => this.artifactsByName[name];
    },
    artifactsOfType(): (type: string) => Artifact[] {
      return (type) => this.artifacts.filter((artifact) => artifact.type == type);
    },
    models(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "model");
    },
    datasets(): Artifact[] {
      return this.artifacts.filter((artifact) => artifact.type == "dataset");
    },
    isHead(): (version: ArtifactVersion) => boolean | undefined {
      return (version) => this.artifact(version.artifact)?.head?.id == version.id;
    },
  },
  actions: {
    async hydrate() {
      const artifacts = (await api.get<Artifact[]>("/artifacts")).data;
      artifacts.forEach(this._addArtifact);
    },
    async dehydrate() {
      this.$reset();
    },

    _addArtifact(artifact: Artifact) {
      this.artifacts.push(artifact);
      this.artifactsByName[artifact.name] = artifact;
      if (artifact.head != null) {
        this._cacheArtifactVersion(artifact.head);
      }
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
          if (this.cachedVersions[key] != null) {
            delete this.cachedVersions[key];
          }
        }
      }
    },

    _cacheArtifactVersion(artifactVersion: ArtifactVersion) {
      const nameVersion = toNameVersion(artifactVersion);
      this.cachedVersions[nameVersion] = artifactVersion;
      return artifactVersion;
    },

    async getVersions(artifactName: string, branch = "main", limit = 10, after?: string) {
      const versionsPaginated = (
        await api.get<LimitPaginatedResult<ArtifactVersion>>(`/artifacts/${artifactName}/versions`, {
          params: { branch, limit, after },
        })
      ).data;
      versionsPaginated.results.forEach(this._cacheArtifactVersion);
      return versionsPaginated;
    },

    async getVersion(artifactName: string, version: string) {
      const artifact = `${artifactName}@${version}`;
      if (this.cachedVersions[artifact] != null) {
        return this.cachedVersions[artifact];
      }
      const artifactVersion = (await api.get<ArtifactVersion>(`/artifacts/${artifactName}/versions/${version}`)).data;
      return this._cacheArtifactVersion(artifactVersion);
    },

    async getVersionByTag(artifactName: string, tag: string) {
      const artifactVersion = (await api.get<ArtifactVersion>(`/artifacts/${artifactName}/tags/${tag}`)).data;
      return this._cacheArtifactVersion(artifactVersion);
    },

    async createArtifact(
      type: "model" | "dataset",
      artifact: Pick<Artifact, "name" | "description" | "tags">,
      initialVersion?: Partial<ArtifactVersion>
    ): Promise<Artifact> {
      const artifactInstance = await api.post<Artifact>(`/${type}s`, artifact).then((response) => response.data);

      // TODO @Robustness: create model and initialize from template should be atomic
      const latestVersion = await api
        .post<ArtifactVersion>(`/${type}s/${name}/versions`, initialVersion)
        .then((response) => response.data);
      artifactInstance.head = latestVersion;

      this._addArtifact(artifactInstance);

      return artifactInstance;
    },

    async commitArtifactVersion(name: string, version: Partial<ArtifactVersion>): Promise<ArtifactVersion> {
      const committedVersion = await api
        .post<ArtifactVersion>(`/artifacts/${name}/versions`, version)
        .then((response) => response.data);
      this._cacheArtifactVersion(committedVersion);
      return committedVersion;
    },
  },
});
