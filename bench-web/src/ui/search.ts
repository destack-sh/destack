import { supergraph } from "@/globals";
import {
  isBenchNodeType,
  isEnumType,
  isNodeType,
  isResourceNode,
  isResourceNodeType,
  isStructType,
  isUnloadedNodeType,
} from "@/language/core/const";
import { ENUM_OPTIONS_BY_VALUE, EnumOption, getEnumOption, getEnumOptions } from "@/language/core/enum";
import { makeExpression } from "@/language/core/expression";
import type { NodeSuperGraph, ReadNodeGraph, TypedNodeKey } from "@/language/core/graph";
import {
  getSubtypeEnum,
  makeType,
  makeTypeConstraint,
  nodeToType,
  typeIdentityEquals,
  type TypeIdentity,
} from "@/language/core/type";
import {
  BenchType,
  BlockType,
  ColorType,
  EnumType,
  ExpressionData,
  ExpressionType,
  FieldType,
  FileType,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeType,
  PrimitiveType,
  ResourceStatus,
  StructType,
  TypeFormat,
  TypeKind,
  ViewType,
  type AnyNodeData,
  type IconData,
  type NodeReferenceData,
  ActionType,
  NodeMode,
} from "@/proto/wire";
import { isNode, makeScope, propertyReference, toNodeRef } from "@/proto/wiring";
import { CURRENT_BENCH_SCOPE } from "@/system/client";
import {
  acquireConnection,
  releaseConnection,
  RemoteSearchConnection,
  SearchConnectionParams,
} from "@/system/connection";
import { benchGraph } from "@/system/space";
import { COMMAND_BUILTIN_IDS_INDEX, IMPLEMENTED_COMMANDS, isCommandEnabled, type Command } from "@/ui/command";
import {
  AVAILABLE_EMOJI_ICONS,
  AVAILABLE_FA_ICONS,
  DEFAULT_ENUM_ICON,
  DEFAULT_MISSING_ICON,
  EmojiIcon,
  emojiIcon,
  fontAwesomeIcon,
  getNodeIcon,
  getNodeTitle,
  type FontAwesomeIcon,
} from "@/ui/icon";
import uFuzzy from "@leeoniya/ufuzzy";
import { tryOnBeforeUnmount, useDebounce } from "@vueuse/core";
import { computed, markRaw, ref, shallowRef, toValue, watch, type MaybeRef, type Ref } from "vue";

export type NodeItem = Omit<NodeReferenceData, "metatype" | "id"> & {
  metatype: "node";
  itemId: string; // per index
  node: AnyNodeData;
  id: string;
  icon: IconData;
  title: string;
  alias?: string;
  path?: string; // the ancestor path to display
  pathToIndex?: string; // alternative path to index for searching (length must match path for highlighting!)
  ancestors: NodeItem[]; // in order of traversal up, excl. self
};
export type CommandItem = Omit<Command, "title"> & {
  metatype: "command";
  itemId: string; // per index
  title: string;
  icon?: IconData;
  text?: string;
  path?: string;
  pathToIndex?: string;
};
export type EnumOptionItem = EnumOption & {
  metatype: "enum-option";
  itemId: string; // per index
};
export type TypeItem = TypeIdentity & {
  metatype: "type";
  itemId: string; // per index
  id: string;
  icon?: IconData;
  title: string;
  text?: string;
  path?: string;
  pathToIndex?: string;
};
export type IconItem = { id: string; title: string; itemId: string; metatype: "icon"; icon: IconData; alias?: string };
export type SearchItem = (NodeItem | CommandItem | EnumOptionItem | TypeItem | IconItem) & {
  itemId: string; // per index
  title: string;
  text?: string;
  color?: ColorType;
  icon?: IconData;
  alias?: string;
  category?: string;
};

export type SearchCandidateInfo = { candidate: string; category: string; index: string };
export type SearchResultInfo = {
  pathMarked?: string;
  titleMarked?: string;
  aliasMarked?: string;
};

/** An index of searchable items (usually wrappers around some 'values'). */
export type SearchIndex<T extends SearchItem> = {
  /** Id and prefix */
  id: string;
  /** Maps a value to a SearchItem (to reverse lookup existing values). */
  getItemFromValue: (value: any) => T | null;
  /** Gets the value from a SearchItem */
  getValueFromItem: (candidate: T) => any;
  /** Produces the current list of candidates. This is non-reactive for search stability & performance. */
  candidates: () => T[];
};

export type SearchOptions = {
  /** Term permutations */
  outOfOrder?: number;
  maxResults?: number;
  highlight?: boolean;
};

const DEFAULT_SEARCH_OPTIONS: Required<SearchOptions> = {
  outOfOrder: 2,
  maxResults: 40,
  highlight: true,
};

const VISIBLE_SEPARATOR = ` / `;
const HIDDEN_SEPARATOR = ` ; `;
const VISIBLE_UNNAMED = `...`;
const HIDDEN_UNNAMED = ` \\ `;

//
// Search
//

// NOTE: :Cleanup: useSearch.indices should type with T, but T is usually a union of different item types,
//  which are distinct per index. So we need multiple Ts for SearchIndex<A>, SearchIndex<B>, etc. How?

/**
 * Search manually defined indices with a query string.
 */
export function useIndexSearch<T extends SearchItem>(search: {
  query: Ref<string>;
  indices: MaybeRef<Record<string, SearchIndex<any>>>;
  isEnabled?: MaybeRef<boolean>;
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

  const candidatesRef = shallowRef<SearchCandidate[]>([]);
  const resultsRef = shallowRef<SearchResult[]>([]);
  const resultsTotal = shallowRef(0);
  let uf: uFuzzy | null = null;
  const subs: (() => void)[] = [];

  function updateCandidates() {
    if (search.isEnabled != null && !toValue(search.isEnabled)) {
      candidatesRef.value = [];
      return;
    }
    const candidates: SearchCandidate[] = [];
    for (const [indexName, index] of Object.entries(toValue(search.indices))) {
      const indexCandidates = index
        .candidates()
        .map((item) => ({ ...item, category: item.category ?? indexName, index: indexName }) as SearchCandidate);
      candidates.push(...indexCandidates);
    }
    candidatesRef.value = candidates;
  }

  function updateResults() {
    if (search.isEnabled != null && !toValue(search.isEnabled)) {
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
    if (uf == null) {
      uf = new uFuzzy({ intraMode: 1 });
    }

    // Build search strings array with title, path and alias for each candidate
    const searchStrings: string[] = [];
    const searchStringMap = new Map<number, { candidate: SearchCandidate; key: "title" | "path" | "alias" }>();

    candidates.forEach((candidate) => {
      searchStrings.push(candidate.title);
      searchStringMap.set(searchStrings.length - 1, { candidate, key: "title" });
      // path if exists
      if ("path" in candidate && candidate.path != null) {
        searchStrings.push(candidate.pathToIndex ?? candidate.path);
        searchStringMap.set(searchStrings.length - 1, { candidate, key: "path" });
      }
      // alias if exists
      if (candidate.alias) {
        searchStrings.push(candidate.alias);
        searchStringMap.set(searchStrings.length - 1, { candidate, key: "alias" });
      }
    });

    const [idxs, info, order] = uf.search(searchStrings, search.query.value, options.outOfOrder);

    // collect
    if (idxs && order) {
      const results = new Map<string, SearchResult>();
      const maxResults = Math.min(options.maxResults, order.length);

      for (let orderIdx = 0; orderIdx < maxResults; orderIdx++) {
        const infoIdx = order[orderIdx];
        const searchStringIdx = idxs[infoIdx];
        const { candidate, key: field } = searchStringMap.get(searchStringIdx)!;

        // Get or create result for this candidate
        let result = results.get(candidate.itemId);
        if (!result) {
          result = { ...candidate } as SearchResult;
          results.set(candidate.itemId, result);
        }

        // highlight the matched field
        if (options.highlight) {
          const ranges = info.ranges[infoIdx] as number[];

          if (field === "title") {
            result.titleMarked = highlight(candidate.title, ranges, {
              start: 0,
              end: candidate.title.length,
            });
          } else if (field === "path" && "path" in candidate && candidate.path) {
            result.pathMarked = highlight(candidate.path, ranges, {
              start: 0,
              end: candidate.path.length,
            });
          } else if (field === "alias" && candidate.alias) {
            result.aliasMarked = highlight(candidate.alias, ranges, {
              start: 0,
              end: candidate.alias.length,
            });
          }
        }
      }

      // count unique candidates that matched
      const uniqueMatchCount = new Set(
        order
          .map((orderIdx) => idxs[orderIdx])
          .map((searchStringIdx) => searchStringMap.get(searchStringIdx)!.candidate.itemId),
      ).size;
      resultsTotal.value = uniqueMatchCount;
      resultsRef.value = Array.from(results.values());
    } else {
      resultsTotal.value = 0;
      resultsRef.value = [];
    }
  }

  // refresh candidates on index change
  subs.push(
    watch(
      () => search.isEnabled == null || [toValue(search.isEnabled), toValue(search.indices)],
      () => {
        updateCandidates();
        updateResults();
      },
      { immediate: true },
    ),
  );

  // update results on query change
  subs.push(watch(search.query, updateResults, { immediate: true }));

  tryOnBeforeUnmount(() => subs.forEach((sub) => sub()));

  return { candidates: candidatesRef, results: resultsRef, resultsTotal, updateCandidates, updateResults };
}

//
// Highlighting
//

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

/** Make remote search parameters for a remote (node) value type */
export function makeRemoteSearchParams(options: {
  query: string;
  valueType: TypeIdentity;
  first: number;
  isEnabled?: boolean;
}): SearchConnectionParams<any> {
  const { query, valueType } = options;
  const queryString = query;
  const nodeType = valueType.benchType;
  if (!isNodeType(nodeType)) {
    throw new Error(`unsupported remote search type: ${nodeType}`);
  }
  const nodeProperties = NODE_PROPERTY_ENUM_BY_TYPE[nodeType];
  // filter
  const filterClauses: ExpressionData[] = [];
  if (queryString.length > 0) {
    // query string filtering
    for (const key of ["slug", "name", "title"]) {
      if (key in nodeProperties) {
        const propertyPtr = propertyReference(nodeType, nodeProperties[key]);
        const clause = makeExpression({ type: ExpressionType.MATCHES, propertyPtr, value: queryString });
        filterClauses.push(clause);
      }
    }
  }
  if (isResourceNodeType(nodeType)) {
    // exclude decommissioned resources
    const clause = makeExpression({
      type: ExpressionType.NOT_EQUALS,
      propertyPtr: propertyReference(nodeType, nodeProperties["status"]),
      value: ResourceStatus.DECOMMISSIONED,
    });
    filterClauses.push(clause);
  }
  const filter =
    filterClauses.length > 0 ? makeExpression({ type: ExpressionType.OR, clauses: filterClauses }) : undefined;
  // sort
  const sort: ExpressionData[] = [
    makeExpression({
      type: ExpressionType.DESCENDING,
      propertyPtr: propertyReference(nodeType, nodeProperties["createdAt"]),
    }),
  ];
  // params
  const scope = isBenchNodeType(nodeType) ? CURRENT_BENCH_SCOPE.value : makeScope({});
  const params: SearchConnectionParams<any> = {
    nodeType,
    scope,
    baseTypePtr: valueType.baseTypePtr,
    filter,
    sort,
    first: options.first,
    isEnabled: options.isEnabled,
  };
  return params;
}

const VALUE_SEARCH_FIRST = 20;
const VALUE_SEARCH_DEBOUNCE = 100;

/**
 * Search dynamically built indices that match a value type and query string.
 * NOTE :Incomplete: it would be nice to support custom search clauses for remote searches
 */
export function useValueSearch(options: {
  query: Ref<string>;
  valueType: Ref<TypeIdentity | undefined | null>;
  index?: MaybeRef<SearchIndex<any> | undefined | null>;
  isEnabled?: MaybeRef<boolean>;
  first?: number;
  debounce?: number;
}) {
  const { query, valueType, isEnabled, first = VALUE_SEARCH_FIRST, debounce = VALUE_SEARCH_DEBOUNCE } = options;
  const queryDebounced = useDebounce(query, debounce);
  const remoteGraphIndex: Ref<SearchIndex<any> | null> = shallowRef(null);
  let remoteConnection: RemoteSearchConnection<any> | null = null;
  const remoteUpdateTrigger = shallowRef(0);
  const isLoading = ref(false);

  // maintain remote connection (if needed)
  let lastRemoteQuery: string = "";
  let lastRemoteValueType: TypeIdentity | null = null;
  let lastRemoteTotal: number = 0;
  watch(
    [queryDebounced, valueType, () => toValue(isEnabled), remoteUpdateTrigger],
    async () => {
      // release old remote connection
      if (remoteConnection != null) {
        releaseConnection(remoteConnection);
        remoteConnection = null;
      }

      // acquire new remote connection if needed
      if (
        (isEnabled == null || toValue(isEnabled)) &&
        isNodeType(valueType.value?.benchType) &&
        isUnloadedNodeType(valueType.value?.benchType)
      ) {
        // acquire/update remote connection
        if (valueType.value.baseTypePtr != null) {
          const baseType = supergraph.get(valueType.value.baseTypePtr);
          if (baseType == null) {
            return; // missing base type, so no search possible
          }
        }
        const query = queryDebounced.value.trim();
        if (
          lastRemoteValueType != null &&
          typeIdentityEquals(lastRemoteValueType, valueType.value) &&
          query.startsWith(lastRemoteQuery) &&
          lastRemoteTotal < first
        ) {
          return; // no need to update search, already narrowed down with previous search
        }
        // update remote connection
        const params = makeRemoteSearchParams({ query, valueType: valueType.value, first, isEnabled: true });
        isLoading.value = true;
        try {
          const connection = await acquireConnection("search", { name: `picker.search` }, params);
          remoteConnection = connection as RemoteSearchConnection<any>;
          const graph = connection.result.value?.graphComposite!;
          const nodeType = valueType.value.benchType as unknown as NodeType;
          remoteGraphIndex.value = graphIndex({
            id: "graph",
            graph,
            metatypes: [nodeType],
            filter: (node) => !isHiddenBuiltinNodeItem(node),
          });
          lastRemoteQuery = queryDebounced.value;
          lastRemoteValueType = valueType.value;
          lastRemoteTotal = connection.result.value?.rootsPtr.value.length ?? 0;
        } finally {
          isLoading.value = false;
        }
      } else {
        // release remote connection
        remoteGraphIndex.value = null;
        lastRemoteQuery = "";
      }
    },
    { immediate: true },
  );
  function update() {
    // TODO :UX: refresh search on first open? (somehow re-fetch connection?)
  }
  tryOnBeforeUnmount(() => {
    if (remoteConnection != null) {
      releaseConnection(remoteConnection);
      remoteConnection = null;
    }
  });

  // figure out index
  const index: Ref<SearchIndex<any> | null> = computed(() => {
    if (toValue(options.index) != null) {
      return toValue(options.index)!;
    } else if (isEnumType(options.valueType.value?.benchType)) {
      // regular enum
      return enumIndex({ id: "enum", enumType: options.valueType.value.benchType });
    } else if (options.valueType.value?.kind == TypeKind.NODE || isNodeType(options.valueType.value?.benchType)) {
      // node from (some) graph
      if (remoteGraphIndex.value != null) {
        // remote graph
        return remoteGraphIndex.value;
      } else if (
        ([TypeKind.NODE, TypeKind.BASED_NODE].includes(valueType.value?.kind!) && valueType.value?.benchType == null) ||
        (isNodeType(valueType.value?.benchType) && isUnloadedNodeType(valueType.value?.benchType))
      ) {
        // remote supergraph
        return supergraphIndex({
          id: "supergraph",
          supergraph,
          metatypes: valueType.value?.constraint?.nodeTypes,
          filter: (node) => !isHiddenBuiltinNodeItem(node),
        });
      } else {
        // local graph
        const nodeType = valueType.value?.benchType as unknown as NodeType;
        const graph = benchGraph.nodeTypes.has(nodeType) ? benchGraph : benchGraph;
        let roots: AnyNodeData[] | undefined = undefined;
        let metatypes: NodeType[] = [];
        const subtypes: number[] | undefined = valueType.value?.constraint?.nodeSubtypes;
        if (valueType.value?.baseTypePtr != null) {
          // based node
          const base = graph.get(valueType.value.baseTypePtr);
          if (base != null) roots = [base];
        } else if ((valueType.value?.constraint?.nodeScopePtr?.length ?? 0) > 0) {
          roots = valueType.value!.constraint!.nodeScopePtr.map((r) => graph.get(r)).filter((r) => r != null);
        }
        if (valueType.value?.benchType != null) {
          metatypes = [nodeType];
        } else if ((valueType.value?.constraint?.nodeTypes?.length ?? 0) > 0) {
          metatypes = valueType.value!.constraint!.nodeTypes;
        } else {
          metatypes = [NodeType.BLOCK, NodeType.ACTION, NodeType.FIELD, NodeType.VIEW];
        }
        const filter =
          subtypes != null
            ? (node: AnyNodeData) => subtypes.includes((node as any).type) && !isHiddenBuiltinNodeItem(node)
            : (node: AnyNodeData) => !isHiddenBuiltinNodeItem(node);
        return graphIndex({ id: "graph", graph, roots, metatypes, filter });
      }
    } else if (options.valueType.value?.benchType == BenchType.TYPE) {
      // some type
      return TYPE_INDEX;
    } else {
      return null;
    }
  });

  const { candidates, results, resultsTotal } = useIndexSearch<SearchItem>({
    query: options.query,
    indices: computed(() => {
      if (index.value == null) return {};
      else return { [index.value.id]: index.value };
    }),
    isEnabled: options.isEnabled,
  });

  /** Get the SearchItem from its value */
  function getItemFromValue(value: any): SearchItem | null {
    const item = index.value?.getItemFromValue(value);
    if (item != null) return item;
    return null;
  }

  /** Get the value from a SearchItem */
  function getValueFromItem(item: SearchItem): any {
    const value = index.value?.getValueFromItem(item);
    if (value != null) return value;
    return null;
  }

  return { index, candidates, results, resultsTotal, isLoading, update, getItemFromValue, getValueFromItem };
}

//
// Graph index
//

/** Make a NodeItem from a Node */
function nodeItemFromNode(
  indexId: string,
  graph: ReadNodeGraph | NodeSuperGraph,
  value: AnyNodeData | TypedNodeKey<any>,
  ancestors: NodeItem[] = [],
  options: { skipDepth?: number } = {},
): NodeItem | null {
  const node = isNode(value) ? value : graph.get(value);
  if (node == null) return null;

  // 'title'
  const title = getNodeTitle(node) ?? "";

  // compose path
  const pathParts = [];
  for (let i = ancestors.length - 1 - (options.skipDepth ?? 0); i >= 0; i--) {
    const ancestor = ancestors[i];
    if (ancestor.title != null && ancestor.title.length > 0) {
      // handle bench:package special case
      if (i > 0 && isNode(ancestor.node, NodeType.BENCH) && isNode(ancestors[i - 1].node, NodeType.PACKAGE)) {
        const pkg = ancestors[i - 1].node;
        const bench = ancestor.node;
        if (bench.mainPackagePtr?.id == pkg.id) {
          pathParts.push(ancestor.title);
        } else {
          pathParts.push(`${ancestor.title}:${ancestors[i - 1].title}`);
        }
        i--; // skip the package node since we've included it
        continue;
      }
      pathParts.push(ancestor.title);
    }
  }
  const path = pathParts.map((p) => p ?? VISIBLE_UNNAMED).join(VISIBLE_SEPARATOR);
  const pathToIndex = pathParts.map((p) => p ?? HIDDEN_UNNAMED).join(HIDDEN_SEPARATOR);

  // make item
  const item: NodeItem = {
    ...(toNodeRef(node)! as NodeReferenceData & { id: string }),
    metatype: "node",
    itemId: `${indexId}-${value.id}`,
    node,
    title,
    path,
    pathToIndex,
    icon: getNodeIcon(node) ?? DEFAULT_MISSING_ICON,
    ancestors: ancestors,
  };
  return item;
}

/**
 * Walks nodes from a graph and transforms them into search items.
 */
function walkGraph(options: {
  id: string;
  graph: ReadNodeGraph;
  metatypes?: NodeType[];
  roots?: AnyNodeData[];
  skipDepth?: number;
  maxDepth?: number;
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
}): NodeItem[] {
  const { id, graph, metatypes, roots, skipDepth, maxDepth, filter } = options;
  const items: NodeItem[] = [];

  /**
   * Walks the descendants from a node.
   */
  function walkNode(node: AnyNodeData, ancestors: NodeItem[]) {
    // make item
    const ref = toNodeRef(node);
    if (ref.id == null) throw new Error(`node has no id: ${node}`);
    const item = nodeItemFromNode(id, graph, ref, ancestors);
    if (item == null) return;

    // add this item if it matches the filter
    if (
      (metatypes == null || metatypes.includes(node.metatype as unknown as NodeType)) &&
      (filter == null || filter(node, ancestors)) &&
      (skipDepth == null || ancestors.length >= skipDepth)
    ) {
      items.push(item);
    }

    // descend if possible
    const nextAncestors = [item, ...ancestors];
    if (maxDepth == null || ancestors.length < maxDepth) {
      for (const child of graph.getChildren(node)) {
        walkNode(child, nextAncestors);
      }
    }
  }

  // walk from roots
  for (const root of roots ?? graph.roots) {
    walkNode(root, []);
  }

  return items;
}

/** Whether the Node is a hidden builtin node (NOTE :HiddenBuiltinStuff) */
export function isHiddenBuiltinNodeItem(node: AnyNodeData): boolean {
  if (
    (isNode(node, NodeType.BLOCK) || isNode(node, NodeType.PAGE) || isResourceNode(node)) &&
    node.mode == NodeMode.BUILTIN
  ) {
    return true;
  } else {
    return false;
  }
}

/**
 * Search nodes in a graph.
 */
export function graphIndex(idx: {
  id: string;
  graph: ReadNodeGraph;
  metatypes: NodeType[];
  roots?: AnyNodeData[];
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
  skipDepth?: number;
  maxDepth?: MaybeRef<number>;
}): SearchIndex<NodeItem> {
  const index: SearchIndex<NodeItem> = {
    id: idx.id,
    getItemFromValue: (value: TypedNodeKey<any>) => nodeItemFromNode(idx.id, idx.graph, value),
    getValueFromItem: (candidate: NodeItem) => toNodeRef(candidate.node),
    candidates: () => walkGraph({ ...idx, maxDepth: toValue(idx.maxDepth) }),
  };

  return markRaw(index);
}

/**
 * Search nodes in a supergraph.
 */
export function supergraphIndex(idx: {
  id: string;
  supergraph: NodeSuperGraph;
  metatypes?: NodeType[];
  roots?: AnyNodeData[];
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
  skipDepth?: number;
  maxDepth?: MaybeRef<number>;
}): SearchIndex<NodeItem> {
  const index: SearchIndex<NodeItem> = {
    id: idx.id,
    getItemFromValue: (value: TypedNodeKey<any>) => nodeItemFromNode(idx.id, idx.supergraph, value),
    getValueFromItem: (candidate: NodeItem) => toNodeRef(candidate.node),
    candidates: () => {
      // figure out which graphs to search
      let graphs: ReadNodeGraph[];
      if (idx.roots != null) {
        graphs = [];
        for (const root of idx.roots) {
          const link = idx.supergraph.getLink({ id: root.id, nodeType: root.metatype as unknown as NodeType });
          if (link != null && !graphs.includes(link.graph)) {
            graphs.push(link.graph);
          }
        }
      } else {
        graphs = idx.supergraph.liveGraphs;
      }

      // and search them
      const candidatesById: Record<string, NodeItem> = {};
      for (const graph of graphs) {
        const candidates = walkGraph({ ...idx, graph, maxDepth: toValue(idx.maxDepth) });
        for (const candidate of candidates) {
          candidatesById[candidate.itemId] = candidate;
        }
      }
      return Object.values(candidatesById).sort((a, b) => a.title.localeCompare(b.title));
    },
  };
  return markRaw(index);
}
export const SUPERGRAPH_INDEX = supergraphIndex({ id: "supergraph", supergraph: supergraph, metatypes: [] });

//
// Command index
//

/**
 * Search the currently available commands.
 */
export function commandIndex(idx: { id: string } = { id: "command" }): SearchIndex<CommandItem> {
  function getItemFromValue(value: Command): CommandItem {
    return { ...value, title: toValue(value.title), metatype: "command", itemId: `${idx.id}-${value.id}` };
  }
  const index: SearchIndex<CommandItem> = {
    id: idx.id,
    getItemFromValue: getItemFromValue,
    getValueFromItem: (candidate: CommandItem) => candidate.id,
    candidates: () => {
      return IMPLEMENTED_COMMANDS.value
        .filter((a) => isCommandEnabled(a, undefined))
        .sort((a, b) => COMMAND_BUILTIN_IDS_INDEX[a.id] - COMMAND_BUILTIN_IDS_INDEX[b.id])
        .map(getItemFromValue);
    },
  };
  return markRaw(index);
}
export const COMMAND_INDEX = commandIndex();

//
// Enum index
//

/*
 * Search the available options of an enum.
 */
export function enumIndex(idx: { id: string; enumType: EnumType }): SearchIndex<EnumOptionItem> {
  function itemFromEnumOption(enumType: EnumType, value: EnumOption | number): EnumOptionItem | null {
    if (typeof value == "object") {
      // already an enum option
      return { ...value, metatype: "enum-option", itemId: value.id };
    } else {
      // find enum option with value
      const enumOption = getEnumOption(enumType, value);
      if (enumOption != null) return { ...enumOption, metatype: "enum-option", itemId: `${idx.id}-${enumOption.id}` };
    }
    return null;
  }

  const index: SearchIndex<EnumOptionItem> = {
    id: idx.id,
    getItemFromValue: (value: EnumOption | number) => itemFromEnumOption(idx.enumType, value),
    getValueFromItem: (candidate: EnumOptionItem) => candidate.value,
    candidates: () => {
      return getEnumOptions(idx.enumType).map((enumOption) => itemFromEnumOption(idx.enumType, enumOption)!);
    },
  };
  return markRaw(index);
}

//
// Type index
//

function anyNodeTypeItem(idxId: string): TypeItem {
  return {
    metatype: "type",
    kind: TypeKind.NODE,
    id: "node",
    itemId: `${idxId}-node`,
    title: "Node",
    icon: undefined,
    isRequired: false,
    isList: false,
    isSecret: false,
    constraint: undefined,
  };
}

/**
 * Search the available type identities (built-ins plus from graph).
 * NOTE :Architecture: typeIndex shouldn't really be part of search but its full own component? :OverloadedPicker
 */
export function typeIndex(idx: {
  id: string;
  graph: ReadNodeGraph;
  skipDepth?: number;
  maxDepth?: number;
  fieldType?: FieldType;
}): SearchIndex<TypeItem> {
  function typeItemFromTypeIdentity(value: TypeIdentity): TypeItem | null {
    if (value.baseTypePtr != null) {
      const nodeItem = nodeItemFromNode(idx.id, idx.graph, value.baseTypePtr);
      if (nodeItem != null) return typeItemFromNode(nodeItem);
    } else if (value.primitiveType != null) {
      if (value.format != null) {
        const option = ENUM_OPTIONS_BY_VALUE[EnumType.TYPE_FORMAT][value.format];
        if (option != null) return typeItemFromEnumOption(EnumType.TYPE_FORMAT, option);
      }
      const option = ENUM_OPTIONS_BY_VALUE[EnumType.PRIMITIVE_TYPE][value.primitiveType];
      if (option != null) return typeItemFromEnumOption(EnumType.PRIMITIVE_TYPE, option);
    } else if (isStructType(value.benchType)) {
      const option = ENUM_OPTIONS_BY_VALUE[EnumType.STRUCT_TYPE][value.benchType];
      if (option != null) return typeItemFromEnumOption(EnumType.STRUCT_TYPE, option);
    } else if (isNodeType(value.benchType)) {
      const subtypeEnum = getSubtypeEnum(value.benchType);
      if (subtypeEnum != null && value.constraint?.nodeSubtypes?.length == 1) {
        const option = ENUM_OPTIONS_BY_VALUE[subtypeEnum][value.constraint!.nodeSubtypes[0]];
        if (option != null) return typeItemFromEnumOption(subtypeEnum, option);
      }
      const option = ENUM_OPTIONS_BY_VALUE[EnumType.NODE_TYPE][value.benchType];
      if (option != null) return typeItemFromEnumOption(EnumType.NODE_TYPE, option);
    } else if (value.kind == TypeKind.NODE) {
      return anyNodeTypeItem(idx.id);
    }
    return null;
  }

  function typeItemFromEnumOption(enumType: EnumType, option: EnumOption): TypeItem {
    const item: TypeItem = {
      kind: TypeKind.ENUM,
      id: `${enumType}-${option.id}`,
      title: option.title,
      icon: option.icon,
      isRequired: false,
      isList: false,
      isSecret: false,
      metatype: "type",
      itemId: `${idx.id}-${enumType}-${option.id}`,
    };
    if (item.icon == null) item.icon = DEFAULT_ENUM_ICON;
    if (enumType == EnumType.PRIMITIVE_TYPE) {
      item.primitiveType = option.value as PrimitiveType;
      item.kind = TypeKind.PRIMITIVE;
    } else if (enumType == EnumType.TYPE_FORMAT) {
      item.primitiveType = Math.floor((option.value as number) / 100) as PrimitiveType;
      item.kind = TypeKind.PRIMITIVE;
      item.format = option.value as TypeFormat;
    } else if (
      enumType == EnumType.NODE_TYPE ||
      enumType == EnumType.OBJECT_TYPE ||
      enumType == EnumType.BENCH_TYPE ||
      enumType == EnumType.STRUCT_TYPE
    ) {
      item.benchType = option.value as BenchType;
      item.kind = isStructType(option.value) ? TypeKind.STRUCT : TypeKind.NODE;
    } else if (enumType == EnumType.FILE_TYPE) {
      item.title = option.title + " File";
      item.kind = TypeKind.NODE;
      item.benchType = BenchType.FILE;
      item.constraint = makeTypeConstraint({ nodeSubtypes: [option.value as FileType] });
    } else if (enumType == EnumType.BLOCK_TYPE) {
      item.title = option.title + " Block";
      item.kind = TypeKind.NODE;
      item.benchType = BenchType.BLOCK;
      item.constraint = makeTypeConstraint({ nodeSubtypes: [option.value as BlockType] });
    } else if (enumType == EnumType.ACTION_TYPE) {
      item.title = option.title + " Action";
      item.kind = TypeKind.NODE;
      item.benchType = BenchType.ACTION;
      item.constraint = makeTypeConstraint({ nodeSubtypes: [option.value as ActionType] });
    } else if (enumType == EnumType.VIEW_TYPE) {
      item.title = option.title + " View";
      item.kind = TypeKind.NODE;
      item.benchType = BenchType.VIEW;
      item.constraint = makeTypeConstraint({ nodeSubtypes: [option.value as ViewType] });
    } else {
      throw new Error(`unexpected enum type: ${enumType} (${option.value})`);
    }
    return item;
  }

  function getTypeItemEnumOptions(enumType: EnumType) {
    return getEnumOptions(enumType).map((option) => typeItemFromEnumOption(enumType, option));
  }

  function typeItemFromNode(nodeItem: NodeItem): TypeItem {
    const blockAsType = nodeToType(nodeItem.node);
    const item: TypeItem = {
      ...nodeItem,
      ...blockAsType,
      isList: false,
      isSecret: false,
      metatype: "type",
    };
    return item;
  }

  const index: SearchIndex<TypeItem> = {
    id: idx.id,
    getItemFromValue: typeItemFromTypeIdentity,
    getValueFromItem: (candidate: TypeItem) => candidate,
    candidates: () => {
      // primitives
      const primitiveItems = getTypeItemEnumOptions(EnumType.PRIMITIVE_TYPE);
      const typeFormatItems = getTypeItemEnumOptions(EnumType.TYPE_FORMAT);
      const fileItems = getTypeItemEnumOptions(EnumType.FILE_TYPE);
      const nodeItems = getTypeItemEnumOptions(EnumType.NODE_TYPE);

      // and any type definitions from blocks
      const graphItems: TypeItem[] = walkGraph({
        id: idx.id,
        graph: idx.graph,
        metatypes: [NodeType.CHOICE, NodeType.DATABASE],
        skipDepth: idx.skipDepth,
        maxDepth: idx.maxDepth,
      }).map(typeItemFromNode);

      const allItems = [
        ...primitiveItems,
        typeItemFromEnumOption(EnumType.STRUCT_TYPE, ENUM_OPTIONS_BY_VALUE[EnumType.STRUCT_TYPE][StructType.TEXT]),
        typeItemFromEnumOption(EnumType.STRUCT_TYPE, ENUM_OPTIONS_BY_VALUE[EnumType.STRUCT_TYPE][StructType.CODE]),
        ...typeFormatItems,
        ...fileItems,
        ...graphItems,
        ...nodeItems,
      ];

      return allItems;
    },
  };
  return markRaw(index);
}
export const TYPE_INDEX = typeIndex({ id: "type", graph: benchGraph, skipDepth: 2 });

//
// Icon index
//

function itemFromIcon(value: FontAwesomeIcon | EmojiIcon): IconItem {
  const icon = "faName" in value ? fontAwesomeIcon(value) : emojiIcon(value);
  return {
    ...value,
    metatype: "icon",
    itemId: `icon-${value.id}`,
    icon: icon,
    alias: value.aliases?.join(HIDDEN_SEPARATOR),
  };
}
const AVAILABLE_FA_ICONS_ITEMS: IconItem[] = AVAILABLE_FA_ICONS.map(itemFromIcon);
const AVAILABLE_EMOJI_ICONS_ITEMS: IconItem[] = AVAILABLE_EMOJI_ICONS.map(itemFromIcon);

/*
 * Search available icons.
 */
export function iconIndex(id: string, items: IconItem[]): SearchIndex<IconItem> {
  const index: SearchIndex<IconItem> = {
    id,
    getItemFromValue: itemFromIcon,
    getValueFromItem: (candiate: IconItem) => candiate,
    candidates: () => items,
  };
  return markRaw(index);
}

export const FONT_AWESOME_ICON_INDEX = iconIndex("icon-fa", AVAILABLE_FA_ICONS_ITEMS);
export const EMOJI_ICON_INDEX = iconIndex("icon-emoji", AVAILABLE_EMOJI_ICONS_ITEMS);
export const ICON_INDEX = iconIndex("icon", [...AVAILABLE_FA_ICONS_ITEMS, ...AVAILABLE_EMOJI_ICONS_ITEMS]);
