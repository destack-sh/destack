import type { AnyNodeData, ObjectType, IconData, NodeReferenceData, NodeType, EnumType } from "@/proto/wire";
import { describeNode, toNodeReference } from "@/proto/wiring";
import { ACTION_BUILTIN_IDS_INDEX, IMPLEMENTED_ACTIONS, type Action } from "@/system/action";
import type { ReadNodeGraph } from "@/system/graph";
import { markRaw, shallowRef, type Ref, watch, type MaybeRef, toRef, toValue, getCurrentInstance } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { AVAILABLE_FA_ICONS, getNodeIcon, type IconMetadata } from "@/system/icon";
import { getEnumOptions, type EnumOption } from "@/system/lang";
import { tryOnBeforeUnmount } from "@vueuse/core";

export type NodeItem = Omit<NodeReferenceData, "metatype" | "id"> & {
  metatype: "node";
  node: AnyNodeData;
  id: string;
  path: string; // the ancestor path to display
  pathToIndex?: string; // alternative path to index for searching (length must match path for highlighting!)
  ancestors: NodeItem[]; // in order of traversal up, excl. self
  icon: IconData;
  title: string;
};
export type ActionItem = Omit<Action, "title"> & {
  metatype: "action";
  title: string;
  path?: string;
  pathToIndex?: string;
};
export type EnumOptionItem = EnumOption & {
  metatype: "enum-option";
};
export type IconItem = IconMetadata & { metatype: "icon" };
export type SearchItem = (NodeItem | ActionItem | EnumOptionItem | IconItem) & { title: string; category?: string };

export type SearchCandidateInfo = { candidate: string; category: string; index: string };

export type SearchResultInfo = {
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

export type SearchOptions = {
  /** Term permutations */
  maxResults?: number;
  highlight?: boolean;
  outOfOrder?: number;
};

const DEFAULT_SEARCH_OPTIONS: Required<SearchOptions> = {
  outOfOrder: 2,
  maxResults: 40,
  highlight: true,
};

const HIDDEN_SEPARATOR = ` ; `;
const VISIBLE_SEPARATOR = ` / `;
const VISIBLE_UNNAMED = `...`;
const HIDDEN_UNNAMED = ` \\ `;

/**
 * Search nodes in a graph.
 */
export function graphIndex(toIndex: {
  graph: ReadNodeGraph;
  metatypes: NodeType[];
  roots?: AnyNodeData[];
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
  skipDepth?: number;
  maxDepth?: MaybeRef<number>;
}): SearchIndex<NodeItem> {
  const maxDepthRef = toRef(toIndex.maxDepth) as Ref<number | undefined>;
  /**
   * Walks the descendants from a node.
   */
  function walkGraph(node: AnyNodeData, ancestors: NodeItem[]): NodeItem[] {
    // title is composed of nodes in path
    const pathParts = [];
    for (let i = ancestors.length - 1 - (toIndex.skipDepth ?? 0); i >= 0; i--) {
      pathParts.push(ancestors[i].title);
    }
    const path = pathParts.map((p) => p ?? VISIBLE_UNNAMED).join(VISIBLE_SEPARATOR);
    const pathToIndex = pathParts.map((p) => p ?? HIDDEN_UNNAMED).join(HIDDEN_SEPARATOR); // lengths must match for highlighting
    const ref = toNodeReference(node);

    if (ref.id == null) throw new Error(`node has no id: ${node}`);

    // make item
    const item: NodeItem = {
      ...(ref as NodeReferenceData & { id: string }),
      metatype: "node",
      node,
      path,
      pathToIndex,
      title: (node as any).title ?? (node as any).name ?? "",
      icon: getNodeIcon(node),
      ancestors: ancestors,
    };
    const items = [];
    if (
      toIndex.metatypes.includes(node.metatype as unknown as NodeType) &&
      (toIndex.filter == null || toIndex.filter(node, ancestors)) &&
      (toIndex.skipDepth == null || ancestors.length >= toIndex.skipDepth)
    )
      items.push(item);

    // descend
    const nextAncestors = [item, ...ancestors];
    if (maxDepthRef.value == null || ancestors.length < maxDepthRef.value) {
      for (const child of toIndex.graph.getChildren(node)) {
        items.push(...walkGraph(child, nextAncestors));
      }
    }

    return items;
  }

  const index: SearchIndex<NodeItem> = {
    candidates: () => {
      const candidates: NodeItem[] = [];
      const roots = toIndex.roots ?? toIndex.graph.roots;
      for (const root of roots) {
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
        .filter((a) => a.isEnabled == null || toValue(a.isEnabled))
        .sort((a, b) => ACTION_BUILTIN_IDS_INDEX[a.id] - ACTION_BUILTIN_IDS_INDEX[b.id])
        .map((a) => ({ ...a, title: toValue(a.title), metatype: "action" }) as ActionItem),
  };
  return markRaw(index);
}

/*
 * Search the available options of an enum.
 */
export function enumIndex(enumTypes: EnumType[]): SearchIndex<EnumOptionItem> {
  const index: SearchIndex<EnumOptionItem> = {
    candidates: () =>
      enumTypes
        .flatMap((enumType) => getEnumOptions(enumType))
        .map((option) => ({ ...option, metatype: "enum-option" })),
  };
  return markRaw(index);
}

/*
 * Search available icons.
 */
const AVAILABLE_FA_ICONS_ITEMS = AVAILABLE_FA_ICONS.map((i) => ({
  metatype: "icon",
  ...i,
  path: i.alias.join(HIDDEN_SEPARATOR),
})) as IconItem[];
export function iconIndex(): SearchIndex<IconItem> {
  const index: SearchIndex<IconItem> = {
    candidates: () => AVAILABLE_FA_ICONS_ITEMS,
  };
  return markRaw(index);
}

// NOTE: :Cleanup: useSearch.indices should type with T, but T is usually a union of different item types,
//  which are distinct per index. So we need multiple Ts for SearchIndex<A>, SearchIndex<B>, etc. How?
export function useSearch<T extends SearchItem>(search: {
  query: Ref<string>;
  indices: MaybeRef<Record<string, SearchIndex<any>>>;
  isEnabled: Ref<boolean>;
  options?: SearchOptions;
}): {
  candidates: Ref<(T & SearchCandidateInfo)[]>;
  results: Ref<(T & SearchCandidateInfo & SearchResultInfo)[]>;
  resultsTotal: Ref<number>;
  updateCandidates: () => void;
  updateResults: () => void;
} {
  type SearchCandidate = T & SearchCandidateInfo;
  type SearchResult = T & SearchCandidateInfo & SearchResultInfo;

  const indicesRef = toRef(search.indices) as Ref<Record<string, SearchIndex<T>>>;
  const candidatesRef = shallowRef<SearchCandidate[]>([]);
  const resultsRef = shallowRef<SearchResult[]>([]);
  const resultsTotal = shallowRef(0);
  const uf = new uFuzzy({ intraMode: 1 });
  const subs: (() => void)[] = [];

  function updateCandidates() {
    if (!search.isEnabled.value) {
      candidatesRef.value = [];
      return;
    }
    const candidates: SearchCandidate[] = [];
    for (const [indexName, index] of Object.entries(indicesRef.value)) {
      const indexCandidates = index.candidates().map(
        (item) =>
          ({
            ...item,
            category: item.category ?? indexName,
            index: indexName,
          }) as SearchCandidate,
      );
      candidates.push(...indexCandidates);
    }
    candidatesRef.value = candidates;
  }

  function updateResults() {
    if (!search.isEnabled.value) {
      resultsRef.value = [];
      resultsTotal.value = 0;
      return;
    }
    const candidates = candidatesRef.value;
    const options = { ...DEFAULT_SEARCH_OPTIONS, ...search.options };
    if (!search.query.value) {
      resultsRef.value = candidates.slice(0, options.maxResults) as SearchResult[];
      resultsTotal.value = candidates.length;
      return;
    }

    // search
    const haystack = candidates.map((c) => getIndexedStr(c).str);
    const [idxs, info, order] = uf.search(haystack, search.query.value, options.outOfOrder, candidates.length);
    // collect
    if (idxs && order) {
      const results: SearchResult[] = [];
      const maxResults = Math.min(options.maxResults, order.length);
      for (let orderIdx = 0; orderIdx < maxResults; orderIdx++) {
        const infoIdx = order[orderIdx];
        const candidate = candidates[idxs[infoIdx]];
        const result = { ...candidate } as SearchResult;
        // highlight
        if (options.highlight) {
          const { str: indexedStr } = getIndexedStr(candidate);
          result.titleMarked = highlight(candidate.title, info.ranges[infoIdx] as any, {
            start: 0,
            end: candidate.title.length,
          });
          if ("path" in candidate && candidate.path != null) {
            result.pathMarked = highlight(candidate.path, info.ranges[infoIdx] as any, {
              start: candidate.title.length + HIDDEN_SEPARATOR.length,
              end: indexedStr.length,
            });
          }
        }
        results.push(result);
      }
      resultsTotal.value = order.length;
      resultsRef.value = results;
    } else {
      resultsTotal.value = 0;
      resultsRef.value = [];
    }
  }

  function getIndexedStr(item: SearchItem): { str: string; isPathIncluded: boolean } {
    if ("path" in item && item.path != null) {
      // index path (which excludes item itself) + title
      return { str: item.title + HIDDEN_SEPARATOR + (item.pathToIndex ?? item.path), isPathIncluded: true };
    } else {
      return { str: item.title, isPathIncluded: false };
    }
  }

  // refresh candidates on index change
  subs.push(watch([search.isEnabled, indicesRef], updateCandidates, { immediate: true }));

  // update results on query change
  subs.push(watch([search.isEnabled, indicesRef, search.query], updateResults, { immediate: true }));

  tryOnBeforeUnmount(() => subs.forEach((sub) => sub()));

  return { candidates: candidatesRef, results: resultsRef, resultsTotal, updateCandidates, updateResults };
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
export function highlightMatches(search: {
  uf: uFuzzy;
  query: string;
  candidates: string[];
  options?: SearchOptions;
}): {
  markedResults: (string | null)[];
  bestMatches: number[];
} {
  const options = { ...DEFAULT_SEARCH_OPTIONS, ...search.options };
  const { uf, query, candidates } = search;
  const [idxs, info, order] = uf.search(candidates, query, options.outOfOrder);
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
