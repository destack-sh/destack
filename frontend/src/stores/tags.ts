import type { Tag } from "@/types/tags";
import { defineStore } from "pinia";

export const useTagsStore = defineStore("tags", {
  state: () => ({
    tags: [] as Tag[],
    tagsByName: {} as Record<string, Tag>,
  }),
  getters: {
    tag(): (name: string) => Tag | undefined {
      return (name) => this.tagsByName[name];
    },
  },
});
