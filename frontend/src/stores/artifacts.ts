import { api } from "@/api";
import { useUserStore } from "@/stores/user";
import type { LimitPaginatedResult } from "@/types";
import type { Artifact, ArtifactVersion, DatasetRecord } from "@/types/artifacts";
import { toNameVersion } from "@/utils/versioning";
import { defineStore } from "pinia";

export const useArtifactsStore = defineStore("artifacts", {
  state: () => ({
    artifacts: [] as Artifact[],
    artifactsByName: {} as Record<string, Artifact>,
    cachedVersions: {} as Record<string, ArtifactVersion>,
  }),
  getters: {
    apiUrl(): (type: "artifact" | "model" | "dataset", name?: string) => string {
      const userStore = useUserStore();
      return (type, name) => {
        if (name != null) {
          return `/${type}s/${userStore.currentOrganization}/${name}`;
        } else {
          return `/${type}s/${userStore.currentOrganization}`;
        }
      };
    },
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
      const artifacts = (await api.get<Artifact[]>(this.apiUrl("artifact"))).data;
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

    async getVersions(name: string, branch = "main", limit = 10, after?: string) {
      const versionsPaginated = (
        await api.get<LimitPaginatedResult<ArtifactVersion>>(`${this.apiUrl("artifact", name)}/versions`, {
          params: { branch, limit, after },
        })
      ).data;
      versionsPaginated.results.forEach(this._cacheArtifactVersion);
      return versionsPaginated;
    },

    async getVersion(name: string, version: string) {
      const artifact = `${name}@${version}`;
      if (this.cachedVersions[artifact] != null) {
        return this.cachedVersions[artifact];
      }
      const artifactVersion = (await api.get<ArtifactVersion>(`${this.apiUrl("artifact", name)}/versions/${version}`))
        .data;
      return this._cacheArtifactVersion(artifactVersion);
    },

    async createArtifact(
      type: "model" | "dataset",
      artifact: Pick<Artifact, "name" | "description" | "tags">,
      initialVersion?: Partial<ArtifactVersion>
    ): Promise<Artifact> {
      const artifactInstance = await api.post<Artifact>(this.apiUrl(type), artifact).then((response) => response.data);

      // TODO @Robustness: create model and initialize from template should be atomic
      const latestVersion = await api
        .post<ArtifactVersion>(`${this.apiUrl(type, artifact.name)}/versions`, initialVersion)
        .then((response) => response.data);
      artifactInstance.head = latestVersion;

      this._addArtifact(artifactInstance);

      return artifactInstance;
    },

    async commitArtifactVersion(name: string, version: Partial<ArtifactVersion>): Promise<ArtifactVersion> {
      const committedVersion = await api
        .post<ArtifactVersion>(`${this.apiUrl("artifact", name)}/versions`, version)
        .then((response) => response.data);
      this._cacheArtifactVersion(committedVersion);
      return committedVersion;
    },

    async getDatasetRecords(name: string, version?: string): Promise<LimitPaginatedResult<DatasetRecord>> {
      const datasetUrl = version ? `${this.apiUrl("dataset", name)}/versions/${version}` : this.apiUrl("dataset", name);

      return await api
        .get<LimitPaginatedResult<DatasetRecord>>(`${datasetUrl}/records`)
        .then((response) => response.data);
    },
  },
});
