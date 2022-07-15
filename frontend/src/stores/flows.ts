import { api } from "@/api";
import type { Flow, FlowVersion } from "@/types/flows";
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
    async hydrate() {
      this.flows = (await api.get<Flow[]>("/flows")).data;
      this.flows.forEach((flow) => (this.flowsByName[flow.name] = flow));
    },
    async dehydrate() {
      this.$reset();
    },
  },
});
