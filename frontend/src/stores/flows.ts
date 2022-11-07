import { api } from "@/api";
import { getRandomName } from "@/composables/useRandomName";
import { useUserStore } from "@/stores/user";
import type {
  ArtifactConnection,
  Flow,
  FlowArtifactEdge,
  FlowNode,
  FlowNodeConnection,
  FlowNodeEdge,
  FlowVersion,
  LimitPaginatedResult,
} from "@/types";
import { artifactEdges, flowNode } from "@/utils/flows";
import { toNameVersion } from "@/utils/versioning";
import { defineStore } from "pinia";

export const useFlowsStore = defineStore("flows", {
  state: () => ({
    flows: [] as Flow[],
    flowsByName: {} as Record<string, Flow>,
    cachedVersions: {} as Record<string, FlowVersion>,
  }),
  getters: {
    apiUrl(): (name?: string, version?: string) => string {
      const userStore = useUserStore();
      return (name, version) => {
        if (name != null) {
          if (version != null) {
            return `/flows/${userStore.currentOrganization}/${name}/versions/${version}`;
          } else {
            return `/flows/${userStore.currentOrganization}/${name}`;
          }
        } else {
          return `/flows/${userStore.currentOrganization}`;
        }
      };
    },
    lastOpenedFlow(): FlowVersion | undefined {
      function getAccessedDt(flow: Flow): string {
        return flow.head?.created_at || flow.created_at;
      }

      return this.flows
        .filter((flow) => flow.head != null)
        .sort((a, b) => getAccessedDt(b).localeCompare(getAccessedDt(a)))
        .map((flow) => flow.head as FlowVersion)[0];
    },
    flow(): (name: string) => Flow | undefined {
      return (name) => this.flowsByName[name];
    },
    flowExists(): (name: string) => boolean {
      return (name) => this.flow(name) != null;
    },
    isHead(): (version: FlowVersion) => boolean | undefined {
      return (version) => this.flow(version.flow)?.head?.id == version.id;
    },
    newName(): () => string {
      return () => {
        let foundNewName = false;
        let flowName = "";
        while (!foundNewName) {
          flowName = getRandomName();
          if (!this.flowExists(flowName)) {
            foundNewName = true;
          }
        }
        return flowName;
      };
    },

    getFlowNode(): (flow: FlowVersion, id: string) => FlowNode | undefined {
      return (flow, id) => flow.nodes?.find((node) => node.id == id);
    },
    flowNode(): (flow: FlowVersion, id: string) => FlowNode {
      return (flow, id) => flowNode(flow, id);
    },
    getFlowNodeByName(): (flow: FlowVersion, name: string) => FlowNode | undefined {
      return (flow, name) => flow.nodes?.find((node) => node.name == name);
    },
  },
  actions: {
    async hydrate() {
      (await api.get<Flow[]>(this.apiUrl())).data.forEach((flow) => {
        this._addFlow(flow);
        if (flow.head != null) {
          this._cacheFlowVersion(flow.head);
        }
      });
    },
    async dehydrate() {
      this.$reset();
    },

    _addFlow(flow: Flow) {
      this.flows.push(flow);
      this.flowsByName[flow.name] = flow;
      return flow;
    },

    clearVersionCache(flow?: string, version?: string) {
      if (flow == null) {
        // if no flow set, clear everything
        this.cachedVersions = {};
      } else {
        if (version == null) {
          // if no version set, clear all versions for flow
          const staleKeys = Object.keys(this.cachedVersions).filter((key) => key.split("@")[0] == flow);
          staleKeys.forEach((key) => delete this.cachedVersions[key]);
        } else {
          // if flow and version set, clear only that specific version
          const key = `${flow}@${version}`;
          if (this.cachedVersions[key] != null) {
            delete this.cachedVersions[key];
          }
        }
      }
    },

    _cacheFlowVersion(flowVersion: FlowVersion) {
      const nameVersion = toNameVersion(flowVersion);
      this.cachedVersions[nameVersion] = flowVersion;
      return flowVersion;
    },

    async getVersions(name: string, branch = "main", limit = 10, after?: string) {
      const versionsPaginated = (
        await api.get<LimitPaginatedResult<FlowVersion>>(`${this.apiUrl(name)}/versions`, {
          params: { branch, limit, after },
        })
      ).data;
      versionsPaginated.results.forEach(this._cacheFlowVersion);
      return versionsPaginated;
    },

    async getVersion(name: string, version: string) {
      const flow = `${name}@${version}`;
      if (this.cachedVersions[flow] != null) {
        return this.cachedVersions[flow];
      }
      const flowVersion = (await api.get<FlowVersion>(`${this.apiUrl(name)}/versions/${version}`)).data;
      return this._cacheFlowVersion(flowVersion);
    },

    async createFlow(flow: Pick<Flow, "name" | "description" | "tags">): Promise<Flow> {
      return api
        .post<Flow>(this.apiUrl(flow.name), flow)
        .then((response) => response.data)
        .then(this._addFlow);
    },

    async createFlowVersion(
      flow: Flow,
      flowVersion: Pick<FlowVersion, "name" | "description" | "parents" | "tags">
    ): Promise<FlowVersion> {
      return api
        .post<FlowVersion>(`${this.apiUrl(flow.name)}/versions`, flowVersion)
        .then((response) => response.data)
        .then(this._cacheFlowVersion)
        .then((flowVersion) => {
          // if flow doesn't have a head yet, assign it
          if (flow.head == null) {
            flow.head = flowVersion;
          }
          return flowVersion;
        });
    }

  }});
