import type { Operation, OperationError } from "@/proto/services";
import {
  GraphScope,
  type AnyNodeData,
  BenchType,
  type NodeType,
  type NodeTypeMapping,
  NodeReferenceData,
  ReadOptionsData,
  ExpressionData,
  AggregationData,
  NodePropertyEnumByType,
  StructType,
  GetNodesRequest,
  GetNodesResponse,
  SearchNodesResponse,
  WatchEditsRequest,
  SearchNodesRequest,
  WatchEditsResponse,
} from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import { BASED_NODE_TYPES } from "@/system/const";
import type { Transaction } from "@sentry/vue";
import { ref, type MaybeRef, type Ref, shallowRef } from "vue";

export type NodeKey<T extends NodeType> = {
  metatype?: T;
} & ({ id: string } | { ck: string });

export type ReadNodeGraph = {
  get scope(): GraphScope;
  get<T extends NodeType>(node: MaybeRef<NodeKey<T>>): NodeTypeMapping[T];
  getChildren<T extends NodeType>(parent: AnyNodeData, metatype: T): NodeTypeMapping[T][];
  getRef<T extends NodeType>(node: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null>;
  getChildrenRef<T extends NodeType>(parent: MaybeRef<AnyNodeData | null>, metatype: T): Ref<NodeTypeMapping[T][]>;
};

export type WriteNodeGraph = {
  get scope(): GraphScope;
  clear(): void;
  add(node: AnyNodeData): void;
  update(node: AnyNodeData): void;
  remove(node: AnyNodeData): void;
};

export class InMemoryNodeGraph implements ReadNodeGraph, WriteNodeGraph {
  public readonly scope: GraphScope = {};
  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private rootsIds: string[] = [];

  constructor(scope: GraphScope = {}) {
    this.scope = scope;
  }

  clear() {
    this.nodesById = {};
    this.nodesByParentIdAndType = {};
  }

  add(node: AnyNodeData) {
    if (this.nodesById[node.id]) {
      throw new Error(`node [id=${node.id}] already exists`);
    }
    this.nodesById[node.id] = node;

    // add to parent
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      if (!this.nodesById[parentId]) {
        throw new Error(`parent [id=${parentId}] does not exist for node [id=${node.id}]`);
      }
      if (!this.nodesByParentIdAndType[parentId]) {
        this.nodesByParentIdAndType[parentId] = {};
      }
      if (!this.nodesByParentIdAndType[parentId][node.metatype]) {
        this.nodesByParentIdAndType[parentId][node.metatype] = [];
      }
      this.nodesByParentIdAndType[parentId][node.metatype].push(node.id);
    } else {
      this.rootsIds.push(node.id);
    }
  }

  update(node: AnyNodeData) {
    const existing = this.nodesById[node.id];
    if (!existing) {
      throw new Error(`node [id=${node.id}] does not exist`);
    }

    // remove/re-add to update with parent if needed, otherwise just update in place
    if (existing.parentPtr?.id != node.parentPtr?.id) {
      this.remove(existing);
      this.add(node);
    } else {
      this.nodesById[node.id] = node;
    }
  }

  remove(node: AnyNodeData) {
    delete this.nodesById[node.id];

    // remove from parent
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      const nodeIdx = this.nodesByParentIdAndType[parentId][node.metatype].findIndex((n) => n == node.id);
      if (nodeIdx == -1) throw new Error(`node [id=${node.id}] not found in parent [id=${parentId}]`);
      this.nodesByParentIdAndType[parentId][node.metatype].splice(nodeIdx, 1);
    } else {
      const rootIdx = this.rootsIds.findIndex((n) => n == node.id);
      if (rootIdx == -1) throw new Error(`node [id=${node.id}] not found in roots`);
      this.rootsIds.splice(rootIdx, 1);
    }
    // remove any children (recursively)
    for (const childId of this.nodesByParentIdAndType[node.id]?.children || []) {
      const child = this.nodesById[childId];
      this.remove(child);
    }
  }

  get<T extends NodeType>(key: MaybeRef<NodeKey<T>>): NodeTypeMapping[T] {
    return this.nodesById[key.id] as NodeTypeMapping[T];
  }

  findRoots<T extends NodeType>(metatype: T): NodeTypeMapping[T][] {
    return this.rootsIds
      .filter((id) => this.nodesById[id].metatype == (metatype as unknown as BenchType))
      .map((id) => this.nodesById[id] as NodeTypeMapping[T]);
  }

  findRoot<T extends NodeType>(metatype: T): NodeTypeMapping[T] {
    const roots = this.findRoots(metatype);
    if (roots.length > 1) throw new Error(`multiple roots of type ${metatype}`);
    return roots[0] as NodeTypeMapping[T];
  }

  getChildren<T extends NodeType>(parent: AnyNodeData, metatype: T): NodeTypeMapping[T][] {
    const children = this.nodesByParentIdAndType[parent.id]?.[metatype];
    if (!children) return [];
    return children.map((id) => this.nodesById[id]) as NodeTypeMapping[T][];
  }

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null> {
    return ref(null); // nocheckin
  }

  getChildrenRef<T extends NodeType>(parent: Ref<AnyNodeData | null>, metatype: T): Ref<NodeTypeMapping[T][]> {
    return ref([]); // nocheckin
  }
}

export function nodeRef<T extends NodeType>(nodeType: T, id: string): NodeReferenceData {
  return { metatype: BenchType.NODE_REFERENCE, type: nodeType, id };
}

export function toNodeRef(node: AnyNodeData | null): NodeReferenceData | null {
  if (!node) return null;
  const nodeProperties = NodePropertyEnumByType[node.metatype]!;
  const reference: NodeReferenceData = {
    metatype: BenchType.NODE_REFERENCE,
    type: node.metatype as unknown as NodeType,
    id: node.id,
  };
  if ("bench" in nodeProperties && node.parentPtr) {
    reference.benchId = node.parentPtr.benchId;
  }
  if ("ck" in nodeProperties) {
    reference.ck = (node as { ck: string }).ck;
    if (node.metatype in BASED_NODE_TYPES) {
      const base = nodeProperties.getBaseFromData(node);
      if (base) {
        reference.baseCk = base.ck;
        reference.baseBenchId = base.benchId;
      }
    }
  }
  return reference;
}

export function patchReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultStruct(StructType.READ_OPTIONS),
    ...options,
  };
}

const graphsByScopeKey: Ref<{ [scopeKey: string]: InMemoryNodeGraph }> = shallowRef({});

function getScopeKey(scope: GraphScope): string {
  return JSON.stringify(scope);
}

/**
 * Gets the given nodes from the relevant subgraph, fetching as needed.
 * If live, will also ensure that edits for the given nodes are watched.
 */
export function getNodes<T extends NodeType>(
  read: MaybeRef<{
    scope?: GraphScope;
    roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
    options?: Partial<ReadOptionsData>;
    live?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  roots: Ref<NodeTypeMapping[T][]>;
  read: Ref<Operation<GetNodesRequest, GetNodesResponse> | null>;
  watch: Ref<Operation<WatchEditsRequest, WatchEditsResponse> | null>;
  error: Ref<OperationError | null>;
} {
  throw new Error("not yet implemented");
}

export function getChildren<T extends NodeType>(
  parent: MaybeRef<AnyNodeData | null>,
  metatype: T
): {
  graph: ReadNodeGraph;
  children: Ref<NodeTypeMapping[T][]>;
} {
  throw new Error("not yet implemented");
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching as needed.
 * If live, will also ensure that 1) edits for the given nodes are watched and 2) the search is watched.
 */
export function searchNodes<T extends NodeType>(
  search: MaybeRef<{
    nodeType: T;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    first?: number;
    skip?: number;
    after?: string | null;
    options?: Partial<ReadOptionsData>;
    count?: boolean;
    live?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  nodes: Ref<NodeTypeMapping[T][]>;
  search: Ref<Operation<SearchNodesRequest, SearchNodesResponse> | null>;
  page: Ref<{ cursors: string[]; startCursor: string; size: number; total?: number }>;
} {
  throw new Error("not yet implemented");
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching as needed.
 * TODO :Feature: live aggregation
 */
export function aggregateNodes(
  aggregate: MaybeRef<{
    scope?: GraphScope;
    nodeType: NodeType;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    aggregation: ExpressionData;
    enabled?: boolean;
  }>,
): { aggregation: AggregationData } {
  throw new Error("not yet implemented");
}

export function editNodes(transaction?: { scope?: GraphScope }): { graph: ReadNodeGraph; transaction: Transaction } {
  throw new Error("not yet implemented");
}
