import { api } from "@/api";
import { getRandomName } from "@/composables/useRandomName";
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
import { toNameVersion } from "@/utils/versioning";
import { defineStore } from "pinia";

function _flowUrl(flow: FlowVersion) {
  return `/flows/${flow.flow}/versions/${flow.version}`;
}

export const useFlowsStore = defineStore("flows", {
  state: () => ({
    flows: [] as Flow[],
    flowsByName: {} as Record<string, Flow>,
    cachedVersions: {} as Record<string, FlowVersion>,
  }),
  getters: {
    lastOpenedFlow(): FlowVersion | undefined {
      function getAccessedDt(flow: Flow): string {
        return flow.latest_version?.created_at || flow.created_at;
      }

      return this.flows
        .filter((flow) => flow.latest_version != null)
        .sort((a, b) => getAccessedDt(b).localeCompare(getAccessedDt(a)))
        .map((flow) => flow.latest_version as FlowVersion)[0];
    },
    flow(): (name: string) => Flow | undefined {
      return (name) => this.flowsByName[name];
    },
    flowExists(): (name: string) => boolean {
      return (name) => this.flow(name) != null;
    },
    isHead(): (version: FlowVersion) => boolean | undefined {
      return (version) => this.flow(version.flow)?.latest_version?.id == version.id;
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
      return (flow, id) => {
        const node = this.getFlowNode(flow, id);
        if (node == null) {
          throw new Error(`could not find node ${id} in flow ${flow}`);
        }
        return node;
      };
    },
    getFlowNodeByName(): (flow: FlowVersion, name: string) => FlowNode | undefined {
      return (flow, name) => flow.nodes?.find((node) => node.name == name);
    },
  },
  actions: {
    async hydrate() {
      (await api.get<Flow[]>("/flows")).data.forEach(this._addFlow);
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

    async getVersions(flowId: string, branch = "main", limit = 10, after?: string) {
      const versionsPaginated = (
        await api.get<LimitPaginatedResult<FlowVersion>>(`/flows/${flowId}/versions`, {
          params: { branch, limit, after },
        })
      ).data;
      versionsPaginated.results.forEach(this._cacheFlowVersion);
      return versionsPaginated;
    },

    async getVersion(flowId: string, version: string) {
      const flow = `${flowId}@${version}`;
      if (this.cachedVersions[flow] != null) {
        return this.cachedVersions[flow];
      }
      const flowVersion = (await api.get<FlowVersion>(`/flows/${flowId}/versions/${version}`)).data;
      return this._cacheFlowVersion(flowVersion);
    },

    async getVersionByTag(flowId: string, tag: string) {
      const flowVersion = (await api.get<FlowVersion>(`/flows/${flowId}/tags/${tag}`)).data;
      return this._cacheFlowVersion(flowVersion);
    },

    _addFlowNode(flow: FlowVersion, node: FlowNode) {
      if (flow.nodes == null) {
        flow.nodes = [];
      }
      flow.nodes.push(node);
      return node;
    },

    _updateFlowNode(flow: FlowVersion, node: FlowNode) {
      const existingNode = flow.nodes?.find((n) => n.id == node.id);
      if (existingNode != null) {
        Object.assign(existingNode, node);
      }
    },

    _addFlowNodeEdge(flow: FlowVersion, nodeEdge: FlowNodeEdge) {
      if (flow.node_edges == null) {
        flow.node_edges = [];
      }
      flow.node_edges.push(nodeEdge);
      return nodeEdge;
    },

    _addFlowArtifactEdge(flow: FlowVersion, flowEdge: FlowArtifactEdge) {
      if (flow.artifact_edges == null) {
        flow.artifact_edges = [];
      }
      flow.artifact_edges.push(flowEdge);
      return flowEdge;
    },

    _updateFlowArtifactEdge(flow: FlowVersion, flowEdge: FlowArtifactEdge) {
      if (flow.artifact_edges == null) {
        this._addFlowArtifactEdge(flow, flowEdge);
      } else {
        const existingEdge = flow.artifact_edges.find((e) => e.id == flowEdge.id);
        if (existingEdge == null) {
          this._addFlowArtifactEdge(flow, flowEdge);
        } else {
          Object.assign(existingEdge, flowEdge);
        }
      }
      return flowEdge;
    },

    _deleteFlowArtifactEdge(flow: FlowVersion, flowEdgeId: string) {
      if (flow.artifact_edges == null) {
        return;
      }
      flow.artifact_edges = flow.artifact_edges.filter((e) => e.id != flowEdgeId);
    },

    async connectFlowNode(flow: FlowVersion, dependent: FlowNode, connection: FlowNodeConnection) {
      return await this.createFlowNodeEdge(flow, {
        ...connection,
        dependent: dependent.id,
      });
    },

    async setFlowNodeArtifactConnections(
      flow: FlowVersion,
      flowNode: FlowNode,
      connections: ArtifactConnection[],
      remove = true
    ) {
      const existingConnections = this.flowEdges(flow, flowNode, null);

      function sameConnection(c1: ArtifactConnection, c2: ArtifactConnection) {
        return (
          c1.dependency == c2.dependency &&
          c1.connection_type == c2.connection_type &&
          c1.connection_name == c2.connection_name
        );
      }

      function getExistingConnection(connection: ArtifactConnection): FlowArtifactEdge | undefined {
        return existingConnections.find((conn) => sameConnection(conn, connection));
      }

      // create new connections & update existing connections
      for (const connection of connections) {
        const existingConnection = getExistingConnection(connection);
        if (existingConnection == null) {
          // create new
          await this.createFlowArtifactEdge(flow, { ...connection, dependent: flowNode.id });
        } else if (existingConnection != connection) {
          // update existing if changed
          await this.updateFlowArtifactEdge(flow, {
            ...existingConnection,
            ...connection,
          });
        }
      }

      // remove no longer needed connections
      if (remove) {
        const redundantConnections = existingConnections.filter(
          (existingEdge) => !connections.find((e) => sameConnection(e, existingEdge))
        );
        await Promise.all(redundantConnections.map((edge) => this.deleteFlowArtifactEdge(flow, edge.id)));
      }
    },

    async createFlowNode(flow: FlowVersion, flowNode: Pick<FlowNode, "name" | "function_id" | "config_arguments">) {
      return api
        .post<FlowNode>(`${_flowUrl(flow)}/nodes`, flowNode)
        .then((response) => response.data)
        .then((node) => this._addFlowNode(flow, node));
    },

    async updateFlowNode(flow: FlowVersion, flowNode: Partial<FlowNode> & Pick<FlowNode, "id">) {
      return api
        .patch<FlowNode>(`${_flowUrl(flow)}/nodes/${flowNode.id}`, flowNode)
        .then((response) => response.data)
        .then((node) => this._updateFlowNode(flow, node));
    },

    async deleteFlowNode(flow: FlowVersion, flowNode: FlowNode) {
      await api.delete(`${_flowUrl(flow)}/nodes/${flowNode.id}`);
      flow.nodes = flow.nodes?.filter((node) => node.id != flowNode.id) || [];
    },

    async createFlow(flow: Pick<Flow, "name" | "description">): Promise<Flow> {
      return api
        .post<Flow>(`/flows`, flow)
        .then((response) => response.data)
        .then(this._addFlow);
    },

    async createFlowVersion(
      flow: Flow,
      flowVersion: Pick<FlowVersion, "name" | "description" | "parents">
    ): Promise<FlowVersion> {
      return api
        .post<FlowVersion>(`/flows/${flow.name}/versions`, flowVersion)
        .then((response) => response.data)
        .then(this._cacheFlowVersion)
        .then((flowVersion) => {
          // if flow doesn't have a head yet, assign it
          if (flow.latest_version == null) {
            flow.latest_version = flowVersion;
          }
          return flowVersion;
        });
    },

    async createFlowNodeEdge(flow: FlowVersion, flowNodeEdge: Omit<FlowNodeEdge, "id">): Promise<FlowNodeEdge> {
      return api
        .post<FlowNodeEdge>(`/flows/${flow.flow}/versions/${flow.version}/node_edges`, flowNodeEdge)
        .then((response) => response.data)
        .then((edge) => this._addFlowNodeEdge(flow, edge));
    },

    async createFlowArtifactEdge(
      flow: FlowVersion,
      flowArtifactEdge: Omit<FlowArtifactEdge, "id">
    ): Promise<FlowArtifactEdge> {
      return api
        .post<FlowArtifactEdge>(`/flows/${flow.flow}/versions/${flow.version}/artifact_edges`, flowArtifactEdge)
        .then((response) => response.data)
        .then((edge) => this._addFlowArtifactEdge(flow, edge));
    },

    async updateFlowArtifactEdge(
      flow: FlowVersion,
      flowArtifactEdge: Pick<FlowArtifactEdge, "id"> & Partial<FlowArtifactEdge>
    ): Promise<FlowArtifactEdge> {
      return api
        .patch<FlowArtifactEdge>(
          `/flows/${flow.flow}/versions/${flow.version}/artifact_edges/${flowArtifactEdge.id}`,
          flowArtifactEdge
        )
        .then((response) => response.data)
        .then((edge) => this._updateFlowArtifactEdge(flow, edge));
    },

    async deleteFlowArtifactEdge(flow: FlowVersion, id: string): Promise<void> {
      return api
        .patch<void>(`/flows/${flow.flow}/versions/${flow.version}/artifact_edges/${id}`)
        .then((response) => response.data)
        .then(() => this._deleteFlowArtifactEdge(flow, id));
    },
  },
});
