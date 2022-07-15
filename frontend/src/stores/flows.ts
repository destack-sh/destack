import { api } from "@/api";
import type { Flow, FlowNode, FlowNodeEdge, FlowVersion } from "@/types";
import { defineStore } from "pinia";

export const useFlowsStore = defineStore("flows", {
  state: () => ({
    flows: [] as Flow[],
    flowsByName: {} as Record<string, Flow>,
  }),
  getters: {
    flow(): (name: string) => Flow | undefined {
      return (name: string) => this.flowsByName[name];
    },
    isHead(): (version: FlowVersion) => boolean | undefined {
      return (version: FlowVersion) => this.flow(version.flow)?.latest_version?.id == version.id;
    },
  },
  actions: {
    _addFlow(flow: Flow) {
      this.flows.push(flow);
      this.flowsByName[flow.name] = flow;
      return flow;
    },
    async hydrate() {
      (await api.get<Flow[]>("/flows")).data.forEach(this._addFlow);
    },
    async dehydrate() {
      this.$reset();
    },
    async createFlow(flow: Pick<Flow, "name" | "description">): Promise<Flow> {
      return api
        .post<Flow>(`/flows`, flow)
        .then((response) => response.data)
        .then(this._addFlow);
    },
    async createFlowVersion(
      flow: string,
      flowVersion: Pick<FlowVersion, "name" | "description" | "parents">
    ): Promise<FlowVersion> {
      return api
        .post<FlowVersion>(`/flows/${flow}/versions`, flowVersion)
        .then((response) => response.data);
    },
    async createFlowNode(
      flow: string,
      version: string,
      flowNode: Pick<FlowNode, "name" | "function_id" | "config_arguments">
    ): Promise<FlowNode> {
      return api
        .post<FlowNode>(`/flows/${flow}/versions/${version}`, flowNode)
        .then((response) => response.data);
    },
    async createFlowNodeEdge(
      flow: string,
      version: string,
      flowNodeEdge: Omit<FlowNodeEdge, "id">
    ): Promise<FlowNodeEdge> {
      return api
        .post<FlowNodeEdge>(`/flows/${flow}/versions/${version}/node_edges`, flowNodeEdge)
        .then((response) => response.data);
    },
  },
});
