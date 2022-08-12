import { api } from "@/api";
import type { FunctionHandlerSpec, ModelHandlerSpec } from "@/types";
import { defineStore } from "pinia";

export const useMetaStore = defineStore("meta", {
  state: () => ({
    modelHandlers: [] as Array<ModelHandlerSpec>,
    modelHandlersById: {} as Record<string, ModelHandlerSpec>,
    functionHandlers: [] as Array<FunctionHandlerSpec>,
    functionHandlersById: {} as Record<string, FunctionHandlerSpec>,
  }),
  getters: {
    modelHandler(): (handlerId: string) => ModelHandlerSpec {
      return (handlerId) => {
        const handler = this.modelHandlersById[handlerId];
        if (handler == null) {
          throw new Error(`handler wiht id ${handlerId} does not exist`);
        }
        return handler as ModelHandlerSpec;
      };
    },
  },
  actions: {
    async hydrate() {
      const [modelHandlers, functionHandlers] = await Promise.all([
        api.get<ModelHandlerSpec[]>("/meta/models").then((r) => r.data),
        api.get<FunctionHandlerSpec[]>("/meta/functions").then((r) => r.data),
      ]);

      this.modelHandlers = modelHandlers;
      this.modelHandlers.forEach((handler) => (this.modelHandlersById[handler.id] = handler));

      this.functionHandlers = functionHandlers;
      this.functionHandlers.forEach((handler) => (this.functionHandlersById[handler.id] = handler));
    },
    async dehyrate() {
      this.$reset();
    },
  },
});
