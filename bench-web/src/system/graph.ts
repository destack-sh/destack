import { getHostClient, supervisor, type OperationError, type BenchUnaryCall } from "@/proto/services";
import {
  AggregationData,
  BenchType,
  ExpressionData,
  GraphScope,
  NodePropertyEnumByType,
  NodeReferenceData,
  ReadOptionsData,
  StructType,
  type AnyNodeData,
  type IGraphIOClient,
  type NodeType,
  type NodeTypeMapping,
  GetNodesRequest,
  GetNodesResponse,
} from "@/proto/wire";
import { makeDefaultStruct, unwrapSomeNode } from "@/proto/wiring";
import { BASED_NODE_TYPES, getBaseFromNode } from "@/system/const";
import type { Transaction } from "@sentry/vue";
import { computed, ref, shallowRef, toRef, watch, type MaybeRef, type Ref } from "vue";
import { type Operation } from "@/proto/services";

export type NodeKey<T extends NodeType> = Omit<NodeReferenceData, "metatype" | "type"> & { type: T };

export type ReadNodeGraph = {
  get scope(): GraphScope;
  get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][];
  getRef<T extends NodeType>(node: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null>;
  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): Ref<NodeTypeMapping[T][]>;
};

export type WriteNodeGraph = {
  get scope(): GraphScope;
  clear(): void;
  add(node: AnyNodeData): void;
  extend(nodes: AnyNodeData[]): void;
  update(node: AnyNodeData): void;
  remove(node: AnyNodeData): void;
};

/**
 * Core in-memory node graph without regard for hidden nodes or multi-graphs (deleted, archived, etc.).
 */
export class InMemoryNodeGraph implements ReadNodeGraph, WriteNodeGraph {
  public readonly scope: GraphScope = {};
  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByCk: { [ck: string]: string } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private rootsIds: string[] = [];

  constructor(scope: GraphScope = {}) {
    this.scope = scope;
  }

  clear() {
    this.nodesById = {};
    this.nodesByCk = {};
    this.nodesByParentIdAndType = {};
  }

  add(node: AnyNodeData) {
    if (this.nodesById[node.id]) throw new Error(`node [id=${node.id}] already exists`);
    this.nodesById[node.id] = node;
    if ("ck" in node) {
      if (this.nodesByCk[node.ck]) throw new Error(`node [ck=${node.ck}] already exists`);
      this.nodesByCk[node.ck] = node.id;
    }

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

  extend(nodes: AnyNodeData[]) {
    for (const node of nodes) {
      this.add(node);
    }
  }

  update(node: AnyNodeData) {
    const existing = this.nodesById[node.id];
    if (!existing) throw new Error(`node [id=${node.id}] does not exist`);

    // remove/re-add to update with parent if needed, otherwise just update in place
    if (existing.parentPtr?.id != node.parentPtr?.id) {
      this.remove(existing);
      this.add(node);
    } else {
      this.nodesById[node.id] = node;
      if ("ck" in node) this.nodesByCk[node.ck] = node.id;
    }
  }

  remove(node: AnyNodeData) {
    delete this.nodesById[node.id];
    if ("ck" in node) delete this.nodesByCk[node.ck];

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

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const id = "id" in key ? key.id : this.nodesByCk[key.ck!];
    if (!id) return null;
    return this.nodesById[id] as NodeTypeMapping[T];
  }

  getChildren<T extends NodeType>(parent: NodeReferenceData, metatype: T): NodeTypeMapping[T][] {
    const childrenIds = this.nodesByParentIdAndType[parent.id!]?.[metatype];
    if (!childrenIds) return [];
    const children = childrenIds.map((id) => this.nodesById[id]) as NodeTypeMapping[T][];
    // sort if needed
    const properties = NodePropertyEnumByType[metatype as unknown as BenchType]!;
    if ("orderKey" in properties)
      children.sort((a, b) => ((a as any).orderKey ?? "").localeCompareTo((b as any).orderKey));
    return children;
  }

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null> {
    return ref(null); // nocheckin
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): Ref<NodeTypeMapping[T][]> {
    return ref([]); // nocheckin
  }
}

/**
 * A graph composed of multiple (potentially overlapping subgraphs).
 * Nodes are merged from the layers in order, with later layers taking precedence.
 */
export class LayerNodeGraph implements ReadNodeGraph {
  public readonly layers: Ref<ReadNodeGraph[]>;

  constructor(layers: ReadNodeGraph[]) {
    this.layers = shallowRef(layers);
  }

  resetLayers() {
    this.layers.value = [];
  }

  addLayer(layer: ReadNodeGraph) {
    this.layers.value = [...this.layers.value, layer];
  }

  removeLayer(layer: ReadNodeGraph) {
    this.layers.value = this.layers.value.filter((l) => l != layer);
  }

  get scope(): GraphScope {
    if (this.layers.value.length == 0) return {} as GraphScope;
    else return this.layers.value[0].scope;
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    let mergedNode: NodeTypeMapping[T] | null = null;
    for (const layer of this.layers.value) {
      const node = layer.get(key);
      if (node) {
        if (!mergedNode) mergedNode = node;
        else mergedNode = mergeNode(mergedNode, node);
      }
    }
    return mergedNode;
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    const mergedChildrenById: { [id: string]: NodeTypeMapping[T] } = {};
    for (const layer of this.layers.value) {
      if (layer.get(parent)) {
        const children = layer.getChildren(parent, metatype);
        for (const child of children) {
          mergedChildrenById[child.id] = child;
        }
      }
    }
    const children = Object.values(mergedChildrenById);
    // sort if needed
    const properties = NodePropertyEnumByType[metatype as unknown as BenchType]!;
    if ("orderKey" in properties)
      children.sort((a, b) => ((a as any).orderKey ?? "").localeCompareTo((b as any).orderKey));
    return children;
  }

  getRef<T extends NodeType>(node: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null> {
    return ref(null); // nocheckin
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): Ref<NodeTypeMapping[T][]> {
    return ref([]); // nocheckin
  }
}

/**
 * A 'view' of a graph with some nodes filtered out.
 */
export class FilterNodeGraph implements ReadNodeGraph {
  public readonly graph: ReadNodeGraph;
  public readonly includeHidden: boolean;

  constructor(graph: ReadNodeGraph, includeHidden: boolean) {
    this.graph = graph;
    this.includeHidden = includeHidden;
  }

  get scope(): GraphScope {
    return this.graph.scope;
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const node = this.graph.get(key);
    if (!this.includeHidden && (!node || node.deletedAt || node.archivedAt)) return null;
    else return node;
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    const children = this.graph.getChildren(parent, metatype);
    if (!this.includeHidden) return children.filter((n) => !n.deletedAt && !n.archivedAt);
    else return children;
  }

  getRef<T extends NodeType>(node: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null> {
    const ref = this.graph.getRef(node);
    if (this.includeHidden) return ref;
    else return computed(() => (ref.value && !ref.value.deletedAt && !ref.value.archivedAt ? ref.value : null));
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): Ref<NodeTypeMapping[T][]> {
    const ref = this.graph.getChildrenRef(parent, metatype);
    if (this.includeHidden) return ref;
    else return computed(() => ref.value.filter((n) => !n.deletedAt && !n.archivedAt));
  }
}

export function mergeNode<T extends NodeType>(node: NodeTypeMapping[T], patch: NodeTypeMapping[T]): NodeTypeMapping[T] {
  return { ...node, ...patch };
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
      const base = getBaseFromNode(node);
      if (base != null) {
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

function getGraph(scope: GraphScope) {
  const key = getScopeKey(scope);
  if (!graphsByScopeKey.value[key]) {
    graphsByScopeKey.value[key] = new InMemoryNodeGraph(scope);
  }
  return graphsByScopeKey.value[key];
}

async function getGraphClient(scope: GraphScope): Promise<IGraphIOClient> {
  if (scope.benchId) {
    return await getHostClient({ id: scope.benchId });
  } else {
    return supervisor;
  }
}

const CURRENT_SCOPE_KEY = "__CURRENT_SCOPE_KEY__";

/**
 * Gets the given nodes from the relevant subgraph, fetching as needed.
 * If watch, will also ensure that edits for the given nodes are watched.
 */
export function getNodes<T extends NodeType>(
  request: MaybeRef<{
    scope?: GraphScope;
    roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
    options?: Partial<ReadOptionsData>;
    watch?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  roots: Ref<NodeTypeMapping[T][]>;
  error: Ref<OperationError | null>;
} {
  const requestRef = toRef(request);
  const compositeGraph = new LayerNodeGraph([]);
  const error: Ref<OperationError | null> = shallowRef(null);
  const fetchOp: Ref<Operation<GetNodesRequest, GetNodesResponse> | null> = shallowRef(null);
  const watchOp: Ref<Operation<GetNodesRequest, GetNodesResponse> | null> = shallowRef(null);
  let graph: InMemoryNodeGraph | null = null;

  const fetch = async () => {
    const request = requestRef.value;
    if (!request.enabled) return;

    graph = getGraph(request.scope ?? {});

    // fetch if needed
    const client = await getGraphClient(graph.scope);
    try {
      const fetchCall = client.getNodes({
        scope: graph.scope,
        roots: request.roots,
        options: patchReadOptions(request.options || {}),
      });
      fetchOp.value = (fetchCall as BenchUnaryCall<GetNodesRequest, GetNodesResponse>).operation;
      const fetched = await fetchCall.response;
      graph.extend(fetched.nodes.map(unwrapSomeNode));
      error.value = null;
      compositeGraph.resetLayers();
      compositeGraph.addLayer(graph);
    } catch (err) {
      error.value = err as OperationError;
      return;
    }

    // watch if needed
    if (request.watch) {
      /* nocheckin: watch edits */
    } else {
      watchOp.value?.abort?.();
      watchOp.value = null;
    }
  };
  watch(requestRef, fetch, { immediate: true });

  // TODO :Performance :Cleanup: getNodes.roots can be computed more efficiently
  const roots: Ref<NodeTypeMapping[T][]> = computed(() => {
    if (!requestRef.value.enabled || error.value || !graph) return [];
    return requestRef.value.roots.map(
      (root) => graph!.getRef({ type: root.type, id: root.id, ck: root.ck }).value as NodeTypeMapping[T],
    );
  });
  return { graph: compositeGraph, roots, error };
}

export function getChildren<T extends NodeType>(
  parent: MaybeRef<AnyNodeData | null>,
  metatype: T,
): {
  graph: ReadNodeGraph;
  children: Ref<NodeTypeMapping[T][]>;
} {
  throw new Error("not yet implemented");
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching as needed.
 * If watch, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function searchNodes<T extends NodeType>(
  request: MaybeRef<{
    scope?: GraphScope;
    nodeType: T;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    first?: number;
    skip?: number;
    after?: string | null;
    options?: Partial<ReadOptionsData>;
    count?: boolean;
    watch?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  nodes: Ref<NodeTypeMapping[T][]>;
  page: Ref<{ cursors: string[]; startCursor: string; size: number; total?: number }>;
} {
  throw new Error("not yet implemented");
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching as needed.
 * TODO :Feature: watch aggregation
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
): { aggregation: Ref<AggregationData> } {
  throw new Error("not yet implemented");
}

export function editNodes(transaction?: { scope?: GraphScope }): { graph: ReadNodeGraph; transaction: Transaction } {
  throw new Error("not yet implemented");
}
