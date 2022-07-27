import { api } from "@/api";
import type { FunctionHandlerSpec, ModelHandlerSpec, ModelTemplate } from "@/types";
import { defineStore } from "pinia";

export const useMetaStore = defineStore("meta", {
  state: () => ({
    modelHandlers: [] as Array<ModelHandlerSpec>,
    modelHandlersByName: {} as Record<string, ModelHandlerSpec>,
    functionHandlers: [] as Array<FunctionHandlerSpec>,
    functionHandlersByName: {} as Record<string, FunctionHandlerSpec>,
  }),
  getters: {
    modelTemplates(): ModelTemplate[] {
      return [
        {
          name: "spaCy Bundled [Local]",
          handler: this.modelHandlersByName["bench.spacy.bundled"],
        },
        {
          name: "HuggingFace Text Generation [Hosted]",
          handler: this.modelHandlersByName["bench.huggingface.hosted.text_generation"],
        },
        {
          name: "OpenAI Text Generation [Hosted]",
          handler: this.modelHandlersByName["bench.openai.text_generation"],
        },
      ];
    },
    // TODO @Feature: differentiate model connector and model template?
    templateFor(): (handlerId: string) => ModelTemplate | null {
      return (handlerId: string) => {
        const matchingTemplates = this.modelTemplates.filter(
          (template) => template.handler?.name == handlerId
        );
        return matchingTemplates.at(0) || null;
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
      this.modelHandlers.forEach((handler) => (this.modelHandlersByName[handler.name] = handler));

      this.functionHandlers = functionHandlers;
      this.functionHandlers.forEach(
        (handler) => (this.functionHandlersByName[handler.name] = handler)
      );
    },
    async dehyrate() {
      this.$reset();
    },
  },
});
