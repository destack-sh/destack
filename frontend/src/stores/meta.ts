import { api } from "@/api";
import type { ModelHandlerSpec, ModelTemplate } from "@/types";
import { defineStore } from "pinia";

export const useMetaStore = defineStore("meta", {
  state: () => ({
    modelHandlers: [] as Array<ModelHandlerSpec>,
    modelHandlersByName: {} as Record<string, ModelHandlerSpec>,
  }),
  getters: {
    modelTemplates(): ModelTemplate[] {
      return [
        {
          name: "spaCy Bundled [Local]",
          handler: this.modelHandlersByName["bench.spacy.bundled"],
        },
        {
          name: "HuggingFace Hub [Hosted]",
          handler: this.modelHandlersByName["bench.huggingface.hosted"],
        },
        {
          name: "OpenAI Completion [Hosted]",
          handler: this.modelHandlersByName["bench.openai.completion"],
        },
      ];
    },
    // TODO @Feature: differentiate model connector and model template?
    templateFor(): (handlerId: string) => ModelTemplate | null {
      return (handlerId: string) => {
        const matchingTemplates = this.modelTemplates.filter(
          (template) => template.handler.name == handlerId
        );
        return matchingTemplates.at(0) || null;
      };
    },
  },
  actions: {
    async hydrate() {
      this.modelHandlers = (await api.get<ModelHandlerSpec[]>("/meta/models")).data;
      this.modelHandlers.forEach((handler) => (this.modelHandlersByName[handler.name] = handler));
    },
    async dehyrate() {
      this.$reset();
    },
  },
});
