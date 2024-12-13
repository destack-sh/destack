import { blockToType } from "@/language/block";
import { isLocalNodeType, isNodeType, isStructType, TYPE_BLOCK_TYPES } from "@/language/const";
import { EnumOption, getEnumOptions } from "@/language/enum";
import { makeExpression } from "@/language/expression";
import { getSubtypeEnum, makeTypeConstraint, typeIdentityEquals, type TypeIdentity } from "@/language/field";
import type { NodeKey, NodeSuperGraph, ReadNodeGraph } from "@/language/graph";
import {
  BenchType,
  BlockData,
  BlockType,
  ENUM_BY_TYPE,
  EnumType,
  ExpressionData,
  ExpressionType,
  FileType,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeType,
  PrimitiveType,
  StepType,
  TypeFormat,
  TypeKind,
  ViewType,
  type AnyNodeData,
  type IconData,
  type NodeReferenceData,
} from "@/proto/wire";
import { isNode, makeScope, propertyReference, toNodeRef } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { SearchConnectionParams } from "@/system/connection";
import { pkgGraph } from "@/system/space";
import { ACTION_BUILTIN_IDS_INDEX, IMPLEMENTED_ACTIONS, type Action } from "@/ui/action";
import {
  AVAILABLE_FA_ICONS,
  DEFAULT_ENUM_ICON,
  DEFAULT_MISSING_ICON,
  getNodeIcon,
  ICON_BY_TYPE_KIND,
  type IconMetadata,
} from "@/ui/icon";
import uFuzzy from "@leeoniya/ufuzzy";
import { tryOnBeforeUnmount } from "@vueuse/core";
import { markRaw, shallowRef, toRef, toValue, watch, type MaybeRef, type Ref } from "vue";

export type NodeItem = Omit<NodeReferenceData, "metatype" | "id"> & {
  metatype: "node";
  itemId: string; // per index
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
  itemId: string; // per index
  title: string;
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
  path?: string;
  pathToIndex?: string;
};
export type IconItem = IconMetadata & { itemId: string; metatype: "icon"; path?: string; pathToIndex?: string };
export type SearchItem = (NodeItem | ActionItem | EnumOptionItem | TypeItem | IconItem) & {
  itemId: string; // per index
  title: string;
  category?: string;
};

export type SearchCandidateInfo = { candidate: string; category: string; index: string };
export type SearchResultInfo = {
  pathMarked?: string;
  titleMarked?: string;
};

/** An index of searchable items (usually wrappers around some 'values'). */
export type SearchIndex<T extends SearchItem> = {
  /** Id and prefix */
  id: string;
  /** Maps a value to a candidate (to reverse lookup existing values). */
  fromValue: (value: any) => T | null;
  /** Gets the value from a candidate */
  toValue: (candidate: T) => any;
  /** Whether two values from this index are equal */
  valueEquals: (a: any, b: any) => boolean;
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

const VISIBLE_SEPARATOR = ` / `;
const HIDDEN_SEPARATOR = ` ; `;
const VISIBLE_UNNAMED = `...`;
const HIDDEN_UNNAMED = ` \\ `;

/**
 * Walks nodes from a graph and transforms them into search items.
 */
function walkGraph(options: {
  id: string;
  graph: ReadNodeGraph;
  metatypes: NodeType[];
  roots?: AnyNodeData[];
  skipDepth?: number;
  maxDepth?: number;
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
}): NodeItem[] {
  const items: NodeItem[] = [];

  /**
   * Walks the descendants from a node.
   */
  function walkNode(node: AnyNodeData, ancestors: NodeItem[]) {
    // title is composed of nodes in path
    const pathParts = [];
    for (let i = ancestors.length - 1 - (options.skipDepth ?? 0); i >= 0; i--) {
      const ancestor = ancestors[i];
      if (ancestor.title != null && ancestor.title != "") {
        pathParts.push(ancestor.title);
      }
    }
    const path = pathParts.map((p) => p ?? VISIBLE_UNNAMED).join(VISIBLE_SEPARATOR);
    const pathToIndex = pathParts.map((p) => p ?? HIDDEN_UNNAMED).join(HIDDEN_SEPARATOR);
    const ref = toNodeRef(node);

    if (ref.id == null) throw new Error(`node has no id: ${node}`);

    // make item
    let title: string = (node as any).title ?? (node as any).name ?? "";
    if (isNode(node, NodeType.VIEW) && node.nodePtr != null) {
      // take title from wrapped node for node views :ViewNodeTitles :DelegateNodes
      //  (NOTE :UX: maybe we should indicate the real name and index that too somehow?)
      const referencedNode = options.graph.get(node.nodePtr);
      if (referencedNode != null) {
        title = (referencedNode as any).title ?? (referencedNode as any).name ?? "";
      }
    }
    const item: NodeItem = {
      ...(ref as NodeReferenceData & { id: string }),
      metatype: "node",
      itemId: `${options.id}-${ref.id}`,
      node,
      path,
      pathToIndex,
      title,
      icon: getNodeIcon(node) ?? DEFAULT_MISSING_ICON,
      ancestors: ancestors,
    };

    // add this item if it matches
    if (
      options.metatypes.includes(node.metatype as unknown as NodeType) &&
      (options.filter == null || options.filter(node, ancestors)) &&
      (options.skipDepth == null || ancestors.length >= options.skipDepth)
    ) {
      items.push(item);
    }

    // descend if possible
    const nextAncestors = [item, ...ancestors];
    if (options.maxDepth == null || ancestors.length < options.maxDepth) {
      for (const child of options.graph.getChildren(node)) {
        walkNode(child, nextAncestors);
      }
    }
  }

  const roots = options.roots ?? options.graph.roots;
  for (const root of roots) {
    walkNode(root, []);
  }

  return items;
}

/** Make a NodeItem from a Node */
function nodeItemFromNode(
  indexId: string,
  graph: ReadNodeGraph | NodeSuperGraph,
  value: NodeKey<any>,
): NodeItem | null {
  const node = graph.get(value);
  if (node == null) return null;
  let title = (node as any).title ?? (node as any).name ?? "";
  if (isNode(node, NodeType.VIEW) && node.nodePtr != null) {
    const referencedNode = graph.get(node.nodePtr); // :ViewNodeTitles
    title = (referencedNode as any).title ?? (referencedNode as any).name ?? "";
  }
  const item: NodeItem = {
    ...(toNodeRef(node)! as NodeReferenceData & { id: string }),
    metatype: "node",
    itemId: `${indexId}-${value.id}`,
    node,
    title: title,
    icon: getNodeIcon(node) ?? DEFAULT_MISSING_ICON,
    ancestors: [], // not needed?
  };
  return item;
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
    fromValue: (value: NodeKey<any>) => nodeItemFromNode(idx.id, idx.graph, value),
    toValue: (candidate: NodeItem) => toNodeRef(candidate.node),
    valueEquals: (a: NodeKey<any>, b: NodeKey<any>) => a.id == b.id || a.ck == b.ck,
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
  metatypes: NodeType[];
  roots?: AnyNodeData[];
  filter?: (node: AnyNodeData, ancestors: NodeItem[]) => boolean;
  skipDepth?: number;
  maxDepth?: MaybeRef<number>;
}): SearchIndex<NodeItem> {
  const index: SearchIndex<NodeItem> = {
    id: idx.id,
    fromValue: (value: NodeKey<any>) => nodeItemFromNode(idx.id, idx.supergraph, value),
    toValue: (candidate: NodeItem) => toNodeRef(candidate.node),
    valueEquals: (a: NodeKey<any>, b: NodeKey<any>) => a.id == b.id || a.ck == b.ck,
    candidates: () => {
      // figure out which graphs to search
      let graphs: ReadNodeGraph[];
      if (idx.roots != null) {
        graphs = [];
        for (const root of idx.roots) {
          const link = idx.supergraph.getLink(root);
          if (link != null && !graphs.includes(link.graph)) {
            graphs.push(link.graph);
          }
        }
      } else {
        graphs = idx.supergraph.graphs;
      }

      // and search them
      const candidates = graphs.flatMap((graph) => walkGraph({ ...idx, graph, maxDepth: toValue(idx.maxDepth) }));
      return candidates;
    },
  };
  return markRaw(index);
}

/**
 * Search the currently available actions.
 */
export function actionIndex(idx: { id: string } = { id: "action" }): SearchIndex<ActionItem> {
  function map(value: Action): ActionItem {
    return { ...value, title: toValue(value.title), metatype: "action", itemId: `${idx.id}-${value.id}` };
  }
  const index: SearchIndex<ActionItem> = {
    id: idx.id,
    fromValue: map,
    toValue: (candidate: ActionItem) => candidate.id,
    valueEquals: (a, b) => a.id === b.id,
    candidates: () =>
      IMPLEMENTED_ACTIONS.value
        .filter((a) => a.isEnabled == null || toValue(a.isEnabled))
        .sort((a, b) => ACTION_BUILTIN_IDS_INDEX[a.id] - ACTION_BUILTIN_IDS_INDEX[b.id])
        .map(map),
  };
  return markRaw(index);
}
export const ACTION_INDEX = actionIndex();

/*
 * Search the available options of an enum.
 */
export function enumIndex(idx: { id: string; enumTypes: EnumType[]; enumValues?: any[] }): SearchIndex<EnumOptionItem> {
  function itemFromEnumOption(enumTypes: EnumType[], value: EnumOption | number): EnumOptionItem | null {
    if (typeof value == "object") {
      return { ...value, metatype: "enum-option", itemId: value.id };
    } else {
      // find enum option
      for (const enumType of enumTypes) {
        if (ENUM_BY_TYPE[enumType][value] != null) {
          const enumOption = getEnumOptions(enumType).find((o) => o.value === value);
          if (enumOption != null)
            return { ...enumOption, metatype: "enum-option", itemId: `${idx.id}-${enumOption.id}` };
        }
      }
    }
    return null;
  }

  const index: SearchIndex<EnumOptionItem> = {
    id: idx.id,
    fromValue: (value: EnumOption | number) => itemFromEnumOption(idx.enumTypes, value),
    toValue: (candidate: EnumOptionItem) => candidate.value,
    valueEquals: (a: EnumOption | number, b: EnumOption | number) => {
      const aValue = typeof a == "object" ? a.value : a;
      const bValue = typeof b == "object" ? b.value : b;
      return aValue === bValue;
    },
    candidates: () => {
      if (idx.enumValues) {
        return idx.enumValues.map((enumValue) => itemFromEnumOption(idx.enumTypes, enumValue)!);
      } else {
        return idx.enumTypes
          .flatMap((enumType) => getEnumOptions(enumType))
          .map((enumOption) => itemFromEnumOption(idx.enumTypes, enumOption)!);
      }
    },
  };
  return markRaw(index);
}

function anyNodeTypeItem(idxId: string): TypeItem {
  return {
    metatype: "type",
    kind: TypeKind.NODE,
    id: "node",
    itemId: `${idxId}-node`,
    title: "Node",
    icon: ICON_BY_TYPE_KIND[TypeKind.NODE],
    isRequired: false,
    isList: false,
    isSecret: false,
    constraint: undefined,
  };
}

/**
 * Search the available type identities (built-ins plus from graph).
 */
export function typeIndex(idx: {
  id: string;
  graph: ReadNodeGraph;
  skipDepth?: number;
  maxDepth?: number;
}): SearchIndex<TypeItem> {
  function typeItemFromTypeIdentity(value: TypeIdentity): TypeItem | null {
    if (value.baseTypePtr != null) {
      const nodeItem = nodeItemFromNode(idx.id, idx.graph, value.baseTypePtr);
      if (nodeItem != null) return typeItemFromNode(nodeItem);
    } else if (value.primitiveType != null) {
      if (value.format != null) {
        const option = getEnumOptions(EnumType.TYPE_FORMAT).find((option) => option.value == value.format);
        if (option != null) return typeItemFromIntrinsic(EnumType.TYPE_FORMAT, option);
      }
      const option = getEnumOptions(EnumType.PRIMITIVE_TYPE).find((option) => option.value == value.primitiveType);
      if (option != null) return typeItemFromIntrinsic(EnumType.PRIMITIVE_TYPE, option);
    } else if (isNodeType(value.benchType)) {
      const subtypeEnum = getSubtypeEnum(value.benchType);
      if (subtypeEnum != null && value.constraint?.nodeSubtypes?.length == 1) {
        const option = getEnumOptions(subtypeEnum).find((option) => option.value == value.constraint!.nodeSubtypes[0]);
        if (option != null) return typeItemFromIntrinsic(subtypeEnum, option);
      }
      const option = getEnumOptions(EnumType.BENCH_TYPE).find((option) => option.value == value.benchType);
      if (option != null) return typeItemFromIntrinsic(EnumType.BENCH_TYPE, option);
    } else if (value.kind == TypeKind.NODE) {
      return anyNodeTypeItem(idx.id);
    }
    return null;
  }

  function typeItemFromIntrinsic(enumType: EnumType, option: EnumOption): TypeItem {
    const item: TypeItem = {
      kind: TypeKind.LITERAL,
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
      // :TypeFormat
      item.primitiveType = Math.floor((option.value as number) / 100) as PrimitiveType;
      item.kind = TypeKind.PRIMITIVE;
      item.format = option.value as TypeFormat;
    } else if (enumType == EnumType.NODE_TYPE || enumType == EnumType.OBJECT_TYPE || enumType == EnumType.BENCH_TYPE) {
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
    } else if (enumType == EnumType.STEP_TYPE) {
      item.title = option.title + " Step";
      item.kind = TypeKind.NODE;
      item.benchType = BenchType.STEP;
      item.constraint = makeTypeConstraint({ nodeSubtypes: [option.value as StepType] });
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

  function getIntrinsicOptions(enumType: EnumType) {
    return getEnumOptions(enumType).map((option) => typeItemFromIntrinsic(enumType, option));
  }

  function typeItemFromNode(nodeItem: NodeItem): TypeItem {
    const blockAsType = blockToType(nodeItem.node as BlockData);
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
    fromValue: typeItemFromTypeIdentity,
    toValue: (candidate: TypeItem) => candidate,
    valueEquals: typeIdentityEquals,
    candidates: () => {
      // primitives
      const primitiveItems = getIntrinsicOptions(EnumType.PRIMITIVE_TYPE);
      const typeFormatItems = getIntrinsicOptions(EnumType.TYPE_FORMAT);
      const fileItems = getIntrinsicOptions(EnumType.FILE_TYPE);
      const blockItems = getIntrinsicOptions(EnumType.BLOCK_TYPE);
      const stepItems = getIntrinsicOptions(EnumType.STEP_TYPE);
      const nodeItems = getIntrinsicOptions(EnumType.BENCH_TYPE);

      // and any type definitions from blocks
      const graphItems: TypeItem[] = walkGraph({
        id: idx.id,
        graph: idx.graph,
        metatypes: [NodeType.BLOCK],
        filter: (node) => {
          const block = node as BlockData;
          return TYPE_BLOCK_TYPES.includes(block.type);
        },
        skipDepth: idx.skipDepth,
        maxDepth: idx.maxDepth,
      }).map(typeItemFromNode);

      const allItems = [
        ...primitiveItems,
        ...typeFormatItems,
        ...fileItems,
        anyNodeTypeItem(idx.id),
        ...graphItems,
        ...blockItems,
        ...stepItems,
        ...nodeItems,
      ];
      return allItems;
    },
  };
  return markRaw(index);
}
export const TYPE_INDEX = typeIndex({ id: "type", graph: pkgGraph, skipDepth: 2 });

function itemFromIcon(value: IconMetadata): IconItem {
  return { ...value, metatype: "icon", itemId: `icon-${value.id}`, path: value.alias.join(HIDDEN_SEPARATOR) };
}
const AVAILABLE_FA_ICONS_ITEMS: IconItem[] = AVAILABLE_FA_ICONS.map(itemFromIcon);
/*
 * Search available icons.
 */
export function iconIndex(id: string = "icon"): SearchIndex<IconItem> {
  const index: SearchIndex<IconItem> = {
    id,
    fromValue: itemFromIcon,
    toValue: (candiate: IconItem) => candiate,
    valueEquals: (a: IconMetadata, b: IconMetadata) => a.faName === b.faName,
    candidates: () => AVAILABLE_FA_ICONS_ITEMS,
  };
  return markRaw(index);
}
export const ICON_INDEX = iconIndex();

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

  const candidatesRef = shallowRef<SearchCandidate[]>([]);
  const resultsRef = shallowRef<SearchResult[]>([]);
  const resultsTotal = shallowRef(0);
  let uf: uFuzzy | null = null;
  const subs: (() => void)[] = [];

  function updateCandidates() {
    if (!search.isEnabled.value) {
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
    if (uf == null) {
      uf = new uFuzzy({ intraMode: 1 });
    }
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
  subs.push(
    watch(
      () => [search.isEnabled.value, toValue(search.indices)],
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
  const scope = isLocalNodeType(nodeType) ? BENCH_SCOPE.value : makeScope({});
  const sort: ExpressionData[] = [
    makeExpression({
      type: ExpressionType.DESCENDING,
      propertyPtr: propertyReference(nodeType, nodeProperties["createdAt"]),
    }),
  ];
  const filter =
    filterClauses.length > 0 ? makeExpression({ type: ExpressionType.OR, clauses: filterClauses }) : undefined;
  const params: SearchConnectionParams<any> = {
    nodeType,
    scope,
    blockPtr: valueType.baseTypePtr,
    filter,
    sort,
    first: options.first,
    isEnabled: options.isEnabled,
  };
  return params;
}
