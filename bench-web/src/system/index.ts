import type { NodeReferenceData } from "@/proto/wire";
import type { Action } from "@/system/action";
import type { Ref } from "vue";

export type NodeInfo = Omit<NodeReferenceData, "metatype"> & {
  metatype: "node";
  path: NodeInfo[];
  name?: string;
  slug?: string;
};
export type ActionInfo = Action & { metatype: "action" };
export type SearchResult = NodeInfo | ActionInfo;

export function useSearch(search: {
  query: Ref<string>;
  enabled?: Ref<boolean>;
}
): { candidates: Ref<SearchResult[]> } {
  throw new Error("not implemented");
}
 