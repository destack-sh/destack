import type { AnyNodeData, IconData, NodeReferenceData, NodeType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { ACTION_BUILTIN_IDS_INDEX, IMPLEMENTED_ACTIONS, type Action } from "@/system/action";
import type { ReadNodeGraph } from "@/system/graph";
import { getNodeTypeIcon } from "@/system/lang";
import { markRaw, shallowRef, type Ref, watch, type MaybeRef, toRef } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";

export type NodeItem = Omit<NodeReferenceData, "metatype" | "id"> & {
  node: AnyNodeData;
  id: string;
  metatype: "node";
  path: string; // the ancestor path to display
  pathIndexed?: string; // alternative path to index for searching (length must match path for highlighting!)
  ancestors: NodeItem[]; // in order of traversal up, excl. self
  icon: IconData;
  title: string;
};
export type ActionItem = Action & { path?: string; pathIndexed?: string; metatype: "action" };
export type SearchItem = (NodeItem | ActionItem) & { title: string; category?: string };

export type SearchCandidate = SearchItem & { candidate: string; category: string };

export type SearchResult = SearchCandidate & {
  pathMarked?: string;
  titleMarked?: string;
};

/** An index of searchable items. */
export type SearchIndex<T extends SearchItem> = {
  /** Produces the current list of candidates. This is non-reactive for search stability & performance. */
  candidates: () => T[];
  /** Enrichs a lazy search item before we turn it into a candidate/result. */
  enrich?: (item: T) => T;
};

const HIDDEN_SEPARATOR = ` ; `;
const VISIBLE_SEPARATOR = ` / `;
const VISIBLE_UNNAMED = `...`;
const HIDDEN_UNNAMED = ` \\ `;

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
      pathParts.push((ancestor as any).title ?? (ancestor as any).name);
    }
    const path = pathParts.map((p) => p ?? VISIBLE_UNNAMED).join(VISIBLE_SEPARATOR);
    const pathIndexed = pathParts.map((p) => p ?? HIDDEN_UNNAMED).join(HIDDEN_SEPARATOR); // lengths must match
    const ref = toNodeReference(node);
    if (ref.id == null) throw new Error(`node has no id: ${node}`);

    // assemble item
    const item: NodeItem = {
      ...(ref as NodeReferenceData & { id: string }),
      metatype: "node",
      node,
      path,
      pathIndexed,
      title: (node as any).title ?? (node as any).name,
      icon: getNodeTypeIcon(node.metatype as unknown as NodeType),
      ancestors: ancestors,
    };
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
      IMPLEMENTED_ACTIONS.value
        .filter((a) => a.enabled == null || a.enabled.value)
        .sort((a, b) => ACTION_BUILTIN_IDS_INDEX[a.id] - ACTION_BUILTIN_IDS_INDEX[b.id])
        .map((a) => ({ ...a, metatype: "action" })),
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
  const uf = new uFuzzy({ intraMode: 1 });

  function getIndexedStr(item: SearchItem): { str: string; isPathIncluded: boolean } {
    if (item.path != null) {
      // index path (which excludes item itself) + title
      return { str: (item.pathIndexed ?? item.path) + HIDDEN_SEPARATOR + item.title, isPathIncluded: true };
    } else {
      return { str: item.title, isPathIncluded: false };
    }
  }

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
      if (search.query.value) {
        const [idxs, info, order] = uf.search(
          candidates.map((c) => getIndexedStr(c).str),
          search.query.value,
        );
        const results: SearchResult[] = [];
        if (idxs && order) {
          // collect results
          for (let orderIdx = 0; orderIdx < order.length; orderIdx++) {
            const infoIdx = order[orderIdx];
            const candidate = candidates[idxs[infoIdx]];
            const result = { ...candidate } as SearchResult;

            // highlight
            const { str: indexedStr } = getIndexedStr(candidate);
            result.titleMarked = highlight(candidate.title, info.ranges[infoIdx] as any, {
              start: indexedStr.length - candidate.title.length,
              end: indexedStr.length,
            });
            if (candidate.path != null) {
              result.pathMarked = highlight(candidate.path, info.ranges[infoIdx] as any, {
                start: 0,
                end: indexedStr.length - candidate.title.length - HIDDEN_SEPARATOR.length,
              });
            }

            results.push(result);
          }
        }
        resultsRef.value = results;
      } else {
        resultsRef.value = candidates;
      }
    },
    { immediate: true },
  );

  return { candidates: candidatesRef, results: resultsRef };
}

/**
 * Highlights a substring of a match. The offset is into the original search string (and thus also the ranges).
 */
export function highlight(
  substr: string,
  ranges: number[], // start0, end0, start1, end1, ...
  offset: { start: number; end: number },
  mark: (strToMark: string) => string = (str) => `<mark>${str}</mark>`,
): string {
  let marked = "";
  let subLast = 0;
  for (let i = 0; i < ranges.length; i += 2) {
    const sourceStart = ranges[i];
    const sourceEnd = ranges[i + 1];
    if (sourceStart >= offset.start && sourceEnd <= offset.end) {
      marked += substr.slice(subLast, sourceStart - offset.start);
      marked += mark(substr.slice(sourceStart - offset.start, sourceEnd - offset.start));
      subLast = sourceEnd - offset.start;
    }
  }
  marked += substr.slice(subLast); // remainder
  return marked;
}

/** Simple search and highlight in plain text haystack */
export function highlightMatches(search: { uf: uFuzzy; query: string; candidates: string[] }): {
  markedResults: (string | null)[];
  bestMatches: number[];
} {
  const { uf, query, candidates } = search;
  const [idxs, info, order] = uf.search(candidates, query);
  const markedResults: (string | null)[] = candidates.map((c) => null);
  let bestMatches: number[] = [];

  if (idxs && order) {
    for (let orderIdx = 0; orderIdx < order.length; orderIdx++) {
      const infoIdx = order[orderIdx];
      const candidate = candidates[idxs[infoIdx]];
      const result = candidate as string;
      const ranges = info.ranges[infoIdx] as number[];
      markedResults[idxs[infoIdx]] = highlight(result, ranges, { start: 0, end: result.length });
    }
    bestMatches = order.map((orderIdx) => idxs[order[orderIdx]]);
  }

  return { markedResults, bestMatches };
}
