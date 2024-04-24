import {
  BenchType,
  BlockData,
  BlockType,
  ENUM_BY_TYPE,
  EnumType,
  NodeType,
  PrimitiveType,
  type AnyNodeData,
  type IconData,
  type NodeReferenceData,
} from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { ACTION_BUILTIN_IDS_INDEX, IMPLEMENTED_ACTIONS, type Action } from "@/system/action";
import type { ReadNodeGraph, NodeKey } from "@/system/graph";
import { AVAILABLE_FA_ICONS, DEFAULT_ENUM_ICON, getNodeIcon, type IconMetadata } from "@/system/icon";
import { TYPE_BLOCK_TYPES, getEnumOptions, type EnumOption } from "@/system/lang";
import type { TypeIdentity } from "@/system/value";
import uFuzzy from "@leeoniya/ufuzzy";
import { tryOnBeforeUnmount } from "@vueuse/core";
import { markRaw, shallowRef, toRef, toValue, watch, type MaybeRef, type Ref } from "vue";

export type NodeItem = Omit<NodeReferenceData, "metatype" | "id"> & {
  metatype: "node";
  node: AnyNodeData;
  id: string;
  icon: IconData;
  title: string;
  path?: string; // the ancestor path to display
  pathToIndex?: string; // alternative path to index for searching (length must match path for highlighting!)
  ancestors: NodeItem[]; // in order of traversal up, excl. self
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
export type TypeItem = TypeIdentity & {
  metatype: "type";
  id: string;
  icon?: IconData;
  title: string;
  path?: string;
  pathToIndex?: string;
};
export type IconItem = IconMetadata & { metatype: "icon"; path?: string; pathToIndex?: string };
export type SearchItem = (NodeItem | ActionItem | EnumOptionItem | TypeItem | IconItem) & {
  title: string;
  category?: string;
};
export type SearchCandidateInfo = { candidate: string; category: string; index: string };
export type SearchResultInfo = {
  pathMarked?: string;
  titleMarked?: string;
};

/** An index of searchable items. */
export type SearchIndex<T extends SearchItem> = {
  /** Maps a value to a candidate. To reverse lookup existing values. */
  fromValue: (value: any) => T | null;
  /** Gets the value from a candidate */
  toValue: (candidate: T) => any;
  /** Produces the current list of candidates. This is non-reactive for search stability & performance. */
  candidates: () => T[];
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

function walkGraph(
  graph: ReadNodeGraph,
  options: {
    metatypes: NodeType[];
    roots?: AnyNodeData[];
    skipDepth?: number;
    maxDepth?: number;
    filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
  },
): NodeItem[] {
  /**
   * Walks the descendants from a node.
   */
  function walkNode(node: AnyNodeData, ancestors: NodeItem[]): NodeItem[] {
    // title is composed of nodes in path
    const pathParts = [];
    for (let i = ancestors.length - 1 - (options.skipDepth ?? 0); i >= 0; i--) {
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
      options.metatypes.includes(node.metatype as unknown as NodeType) &&
      (options.filter == null || options.filter(node, ancestors)) &&
      (options.skipDepth == null || ancestors.length >= options.skipDepth)
    )
      items.push(item);

    // descend
    const nextAncestors = [item, ...ancestors];
    if (options.maxDepth == null || ancestors.length < options.maxDepth) {
      for (const child of graph.getChildren(node)) {
        items.push(...walkNode(child, nextAncestors));
      }
    }

    return items;
  }

  const roots = options.roots ?? graph.roots;
  const items = [];
  for (const root of roots) {
    items.push(...walkNode(root, []));
  }
  return items;
}

function mapNode(graph: ReadNodeGraph, value: NodeKey<any>): NodeItem | null {
  const node = graph.getMaybe(value);
  if (node == null) return null;
  const item: NodeItem = {
    ...(toNodeReference(node)! as NodeReferenceData & { id: string }),
    metatype: "node",
    node,
    title: (node as any).title ?? (node as any).name ?? "",
    icon: getNodeIcon(node),
    ancestors: [], // not needed?
  };
  return item;
}

/**
 * Search nodes in a graph.
 */
export function graphIndex(options: {
  graph: ReadNodeGraph;
  metatypes: NodeType[];
  roots?: AnyNodeData[];
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
  skipDepth?: number;
  maxDepth?: MaybeRef<number>;
}): SearchIndex<NodeItem> {
  const maxDepthRef = toRef(options.maxDepth) as Ref<number | undefined>;

  const index: SearchIndex<NodeItem> = {
    fromValue: (value: NodeKey<any>) => mapNode(options.graph, value),
    toValue: (candidate: NodeItem) => toNodeReference(candidate.node),
    candidates: () => walkGraph(options.graph, { ...options, maxDepth: maxDepthRef.value }),
  };

  return markRaw(index);
}

/**
 * Search the currently available actions.
 */
export function actionIndex(): SearchIndex<ActionItem> {
  function map(value: Action): ActionItem {
    return { ...value, title: toValue(value.title), metatype: "action" };
  }
  const index: SearchIndex<ActionItem> = {
    fromValue: map,
    toValue: (candidate: ActionItem) => candidate.id,
    candidates: () =>
      IMPLEMENTED_ACTIONS.value
        .filter((a) => a.isEnabled == null || toValue(a.isEnabled))
        .sort((a, b) => ACTION_BUILTIN_IDS_INDEX[a.id] - ACTION_BUILTIN_IDS_INDEX[b.id])
        .map(map),
  };
  return markRaw(index);
}

function mapEnumOption(enumTypes: EnumType[], value: EnumOption | number): EnumOptionItem | null {
  if (typeof value == "object") {
    return { ...value, metatype: "enum-option" };
  } else {
    // find enum option
    for (const enumType of enumTypes) {
      if (ENUM_BY_TYPE[enumType][value] != null) {
        const enumOption = getEnumOptions(enumType).find((o) => o.value === value);
        if (enumOption != null) return { ...enumOption, metatype: "enum-option" };
      }
    }
  }
  return null;
}

/*
 * Search the available options of an enum.
 */
export function enumIndex(enumTypes: EnumType[]): SearchIndex<EnumOptionItem> {
  const index: SearchIndex<EnumOptionItem> = {
    fromValue: (value: EnumOption | number) => mapEnumOption(enumTypes, value),
    toValue: (candidate: EnumOptionItem) => candidate.value,
    candidates: () =>
      enumTypes
        .flatMap((enumType) => getEnumOptions(enumType))
        .map((enumOption) => mapEnumOption(enumTypes, enumOption)!),
  };
  return markRaw(index);
}

/**
 * Search the available type identities (built-ins plus from graph).
 */
export function typeIndex(options: {
  graph: ReadNodeGraph;
  skipDepth?: number;
  maxDepth?: number;
}): SearchIndex<TypeItem> {
  const enumTypes = [EnumType.PRIMITIVE_TYPE, EnumType.BENCH_TYPE];

  function map(value: TypeIdentity): TypeItem | null {
    if (value.baseTypePtr != null) {
      const nodeItem = mapNode(options.graph, value.baseTypePtr);
      if (nodeItem != null) return mapFromNode(nodeItem);
    } else if (value.primitiveType != null) {
      const enumOption = getEnumOptions(EnumType.PRIMITIVE_TYPE).find((option) => option.value == value.primitiveType);
      if (enumOption != null) return mapFromOption(EnumType.PRIMITIVE_TYPE, enumOption);
    } else if (value.benchType != null) {
      const enumOption = getEnumOptions(EnumType.BENCH_TYPE).find((option) => option.value == value.benchType);
      if (enumOption != null) return mapFromOption(EnumType.BENCH_TYPE, enumOption);
    }
    return null;
  }

  function mapFromOption(enumType: EnumType, option: EnumOption): TypeItem {
    const item: TypeItem = { ...option, id: `${enumType}-${option.id}`, metatype: "type" };
    if (item.icon == null) item.icon = DEFAULT_ENUM_ICON;
    if (enumType == EnumType.PRIMITIVE_TYPE) item.primitiveType = option.value as PrimitiveType;
    else if (enumType == EnumType.BENCH_TYPE) item.benchType = option.value as BenchType;
    else throw new Error(`unexpected enum type: ${enumType}`);
    return item;
  }

  function mapFromNode(nodeItem: NodeItem): TypeItem {
    const blockType = (nodeItem.node as BlockData).type;
    const item: TypeItem = { ...nodeItem, metatype: "type" };
    if (blockType == BlockType.CHOICE) item.benchType = BenchType.FIELD;
    else if (blockType == BlockType.SIGNAL) item.benchType = BenchType.SIGNAL;
    else if (blockType == BlockType.DATABASE) item.benchType = BenchType.RECORD;
    return item;
  }

  const index: SearchIndex<TypeItem> = {
    fromValue: map,
    toValue: (candidate: TypeItem) => candidate,
    candidates: () => {
      // intrinsic types
      const enumItems: TypeItem[] = enumTypes.flatMap((enumType) =>
        getEnumOptions(enumType).map((option) => mapFromOption(enumType, option)),
      );

      // and any block 'type' definitoin
      const graphItems: TypeItem[] = walkGraph(options.graph, {
        metatypes: [NodeType.BLOCK],
        filter: (node) => {
          return (
            TYPE_BLOCK_TYPES.includes((node as BlockData).type) ||
            (node as BlockData).isProtocol ||
            (node as BlockData).isTemplate
          );
        },
        skipDepth: options.skipDepth,
        maxDepth: options.maxDepth,
      }).map(mapFromNode);

      return [...enumItems, ...graphItems];
    },
  };
  return markRaw(index);
}

function mapIcon(value: IconMetadata): IconItem {
  return { ...value, metatype: "icon", path: value.alias.join(HIDDEN_SEPARATOR) };
}
const AVAILABLE_FA_ICONS_ITEMS: IconItem[] = AVAILABLE_FA_ICONS.map(mapIcon);
/*
 * Search available icons.
 */
export function iconIndex(): SearchIndex<IconItem> {
  const index: SearchIndex<IconItem> = {
    fromValue: mapIcon,
    toValue: (candiate: IconItem) => candiate,
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
