import type { AnyNodeData, IconData, NodeReferenceData, NodeType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { BUILTIN_ACTIONS, type Action } from "@/system/action";
import type { ReadNodeGraph } from "@/system/graph";
import { getNodeIcon } from "@/system/lang";
import { markRaw, shallowRef, type Ref, watch, type MaybeRef, toRef } from "vue";

export type NodeItem = Omit<NodeReferenceData, "metatype" | "id"> & {
  node?: AnyNodeData;
  id: string;
  metatype: "node";
  path: string; // the full path to display
  ancestors: NodeItem[]; // in order of traversal up, excl. self
  icon: IconData;
  title: string;
};
export type ActionItem = Action & { path: string, metatype: "action" };
export type SearchItem = (NodeItem | ActionItem) & { title: string; category?: string };

export type SearchCandidate = SearchItem & { candidate: string; category: string };

export type SearchResult = SearchCandidate & {
  /* TODO :Feature: highlighting */
};

/** An index of searchable items. */
export type SearchIndex<T extends SearchItem> = {
  /** Produces the current list of candidates. This is non-reactive for search stability & performance. */
  candidates: () => T[];
  /** Enrichs a lazy search item before we turn it into a candidate/result. */
  enrich?: (item: T) => T;
};

/**
 * Search nodes in a graph.
 */
export function graphIndex(
  graph: ReadNodeGraph,
  metatypes: NodeType[],
  filter: (node: AnyNodeData, ancestors: NodeItem[]) => boolean,
  maxDepth?: MaybeRef<number>,
): SearchIndex<NodeItem> {
  const maxDepthRef = toRef(maxDepth) as Ref<number | undefined>;
  /**
   * Walks the descendants from a node.
   */
  function walkGraph(node: AnyNodeData, ancestors: NodeItem[]): NodeItem[] {
    // title is composed of nodes in path
    const pathParts = [];
    for (let i = ancestors.length - 1; i >= 0; i--) {
      const ancestor = ancestors[i];
      pathParts.push((ancestor as any).title ?? (ancestor as any).name ?? "...");
    }
    const path = pathParts.join(" / ");
    const ref = toNodeReference(node);
    if (ref.id == null) throw new Error(`node has no id: ${node}`);

    // assemble item
    const item: NodeItem = {
      ...(ref as NodeReferenceData & { id: string }),
      metatype: "node",
      node,
      path,
      title: (node as any).title ?? (node as any).name,
      icon: getNodeIcon(node.metatype as unknown as NodeType),
      ancestors: ancestors,
    };
    console.log(item);
    const items = [];
    if (filter(node, ancestors)) items.push(item);
    const nextAncestors = [item, ...ancestors];
    if (maxDepthRef.value == null || ancestors.length < maxDepthRef.value) {
      for (const metatype of metatypes) {
        for (const child of graph.getChildren(node, metatype)) {
          items.push(...walkGraph(child, nextAncestors));
        }
      }
    }
    return items;
  }

  const index: SearchIndex<NodeItem> = {
    candidates: () => {
      const candidates: NodeItem[] = [];
      for (const root of graph.roots) {
        candidates.push(...walkGraph(root, []));
      }
      return candidates;
    },
  };
  return markRaw(index);
}

/**
 * Search the currently available actions.
 */
export function actionIndex(): SearchIndex<ActionItem> {
  const index: SearchIndex<ActionItem> = {
    candidates: () =>
      Object.values(BUILTIN_ACTIONS.value)
        .filter((a) => a.enabled == null || a.enabled.value)
        .sort((a, b) => a.id.localeCompare(b.id))
        .map((a) => ({ ...a, path: a.title, metatype: "action" })),
  };
  return markRaw(index);
}

export function useSearch(search: {
  query: Ref<string>;
  enabled?: Ref<boolean>;
  indices: Ref<Record<string, SearchIndex<any>>>;
}): {
  candidates: Ref<SearchCandidate[]>;
  results: Ref<SearchResult[]>;
} {
  const candidatesRef = shallowRef<SearchCandidate[]>([]);
  const resultsRef = shallowRef<SearchResult[]>([]);

  watch(
    [search.enabled, search.indices, search.query],
    () => {
      if (!search.enabled?.value) {
        candidatesRef.value = [];
        resultsRef.value = [];
        return;
      }

      // update candidates
      const candidates: SearchCandidate[] = [];
      for (const [category, index] of Object.entries(search.indices.value)) {
        candidates.push(
          ...index.candidates().map((item) => ({
            ...item,
            candidate: item.name ?? item.id,
            category: item.category ?? category,
          })),
        );
      }
      candidatesRef.value = candidates;

      // update results
      const results: SearchResult[] = candidates.filter((candidate) =>
        candidate.title.toLowerCase().includes(search.query.value.toLowerCase()),
      );
      resultsRef.value = results;
    },
    { immediate: true },
  );

  return { candidates: candidatesRef, results: resultsRef };
}
