import { getHostClient, supervisor, type BenchUnaryCall, type Operation, type OperationError } from "@/proto/services";
import {
  AggregationData,
  BenchType,
  ExpressionData,
  GetNodesRequest,
  GetNodesResponse,
  GraphScope,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  ReadOptionsData,
  StructType,
  type AnyNodeData,
  type IGraphIOClient,
  type NodeType,
  type NodeTypeMapping,
  type AnyPropertyType,
} from "@/proto/wire";
import { makeDefaultStruct, unwrapSomeNode } from "@/proto/wiring";
import { BASED_NODE_TYPES, defaultSort, getBaseFromNode } from "@/system/lang";
import { manualComputed, onUnmountedIfComponent } from "@/utils/ref";
import type { Transaction } from "@sentry/vue";
import { computed, shallowRef, toRef, watch, type MaybeRef, type Ref } from "vue";

/** A NodeReference but with proper typing */
export type NodeKey<T extends NodeType> = Omit<NodeReferenceData, "metatype" | "type"> & { type?: T };

/** A read-only reference to something in the graph */
export type GraphRef<T> = Ref<T> & {
  /** Stops tracking */
  stop(): void;
};

/** A node graph with change subscriptions */
export type ReactiveNodeGraph = {
  /** Reactive helpers */
  subscribe(key: { id?: string; ck?: string }, callback: () => void): () => void;
  unsubscribe(key: { id?: string; ck?: string }, callback: () => void): void;
  subscribeChildren<T extends NodeType>(
    parent: { id?: string; ck?: string },
    metatype: T,
    callback: () => void,
  ): () => void;
};

/** A node graph with read methods */
export type ReadNodeGraph = ReactiveNodeGraph & {
  /** The scope contained in this graph */
  get scope(): GraphScope;
  /** Whether this graph is partial */
  readonly isPartial: boolean;
  /** Gets the current node with that key (not reactive) */
  get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  /** Gets the children of the given parent with the given metatype (not reactive) */
  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][];
  /** Gets a reactive reference to the current node with that key */
  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): GraphRef<NodeTypeMapping[T] | null>;
  /** Gets a reactive reference to the children of the given parent with the given metatype */
  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | null>,
    metatype: T,
  ): GraphRef<NodeTypeMapping[T][]>;
};

/** A node graph with write methods */
export type WriteNodeGraph = {
  /** The scope contained in this graph */
  get scope(): GraphScope;
  /** Adds a node to the graph (error if exists) */
  add(node: AnyNodeData): void;
  /** Adds multiple nodes to the graph (error if exists) */
  extend(...nodes: AnyNodeData[]): void;
  /** Updates an existing node in the graph (error if does not exist) */
  update(node: AnyNodeData): void;
  /** Removes a node from the graph (error if does not exist) */
  remove(node: AnyNodeData): void;
};

/**
 * A manually triggered reference to something in the graph.
 * @param get - the computed getter, should update its own dependencies
 * @param stop - the function to stop tracking any dependencies
 * @returns the ref and a trigger to trigger its update (via Vue's reactivity system for batching)
 */
function manualGraphRef<T>(get: () => T, stop: () => void): { ref: GraphRef<T>; trigger: () => void } {
  const manualRef = manualComputed(get);
  const ref = manualRef as unknown as GraphRef<T>;
  ref.stop = stop;
  return { ref, trigger: manualRef.trigger };
}

/**
 * An automatically triggered reference to something in the graph.
 * @param get - a computed getter, should update its own dependencies
 * @param stop - the function to stop tracking any dependencies
 */
function computedGraphRef<T>(get: () => T, stop: () => void): GraphRef<T> {
  const computedRef = computed(get);
  const ref = computedRef as unknown as GraphRef<T>;
  ref.stop = stop;
  return ref;
}

/**
 * Helper mixin for managing reactivity in a graph.
 */
class ReactiveNodeGraphMixin implements ReactiveNodeGraph {
  private subsById: { [id: string]: Array<() => void> } = {};
  private subsByCk: { [ck: string]: Array<() => void> } = {};
  private subsByParentIdAndType: { [parentId: string]: { [type: string]: Array<() => void> } } = {};

  subscribe(key: { id?: string; ck?: string }, callback: () => void): () => void {
    if (key.id) {
      if (!this.subsById[key.id]) this.subsById[key.id] = [];
      this.subsById[key.id].push(callback);
    }
    if (key.ck) {
      if (!this.subsByCk[key.ck]) this.subsByCk[key.ck] = [];
      this.subsByCk[key.ck].push(callback);
    }
    return () => this.unsubscribe(key, callback);
  }

  unsubscribe(key: { id?: string; ck?: string }, callback: () => void) {
    if (key.id) {
      if (this.subsById[key.id]) this.subsById[key.id].splice(this.subsById[key.id].indexOf(callback), 1);
    }
    if (key.ck) {
      if (this.subsByCk[key.ck]) this.subsByCk[key.ck].splice(this.subsByCk[key.ck].indexOf(callback), 1);
    }
  }

  subscribeChildren<T extends NodeType>(
    parent: { id?: string; ck?: string },
    metatype: T,
    callback: () => void,
  ): () => void {
    if (!parent.id) throw new Error("parent must have an id");
    if (!this.subsByParentIdAndType[parent.id]) this.subsByParentIdAndType[parent.id] = {};
    if (!this.subsByParentIdAndType[parent.id][metatype]) this.subsByParentIdAndType[parent.id][metatype] = [];
    this.subsByParentIdAndType[parent.id][metatype].push(callback);
    return () => this.unsubscribeChildren(parent, metatype, callback);
  }

  unsubscribeChildren<T extends NodeType>(parent: { id?: string; ck?: string }, metatype: T, callback: () => void) {
    if (!parent.id) throw new Error("parent must have an id");
    if (this.subsByParentIdAndType[parent.id] && this.subsByParentIdAndType[parent.id][metatype]) {
      this.subsByParentIdAndType[parent.id][metatype].splice(
        this.subsByParentIdAndType[parent.id][metatype].indexOf(callback),
        1,
      );
    }
  }

  notify(node: AnyNodeData) {
    const subscribers = this.subsById[node.id] ?? [];
    for (const sub of subscribers) {
      sub();
    }
    if (node.parentPtr?.id) {
      const subscribers = this.subsByParentIdAndType[node.parentPtr.id]?.[node.metatype] ?? [];
      for (const sub of subscribers) {
        sub();
      }
    }
  }
}

/**
 * Core in-memory node graph without regard for hidden nodes or multi-graphs (deleted, archived, etc.).
 * If 'isPartial', we don't try to maintain local consistency (as this is likely an overlay in a layered graph).
 */
export class NodeGraph extends ReactiveNodeGraphMixin implements ReadNodeGraph, WriteNodeGraph {
  public readonly scope: GraphScope = {};
  public readonly isPartial: boolean = false;
  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByCk: { [ck: string]: string } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private rootsIds: string[] = [];

  constructor(options: { scope?: GraphScope; isPartial?: boolean } = { scope: {}, isPartial: false }) {
    super();
    this.scope = options.scope ?? {};
    this.isPartial = options.isPartial ?? false;
  }

  add(node: AnyNodeData) {
    if (!node.id) throw new Error("node must have an id");
    if (this.nodesById[node.id]) throw new Error(`node [id=${node.id}] already exists`);
    this.nodesById[node.id] = node;
    if ("ck" in node) {
      if (this.nodesByCk[node.ck]) throw new Error(`node [ck=${node.ck}] already exists`);
      this.nodesByCk[node.ck] = node.id;
    }

    // add to parent/roots
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      if (!this.nodesById[parentId] && !this.isPartial) {
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

    this.notify(node);
  }

  extend(...nodes: AnyNodeData[]) {
    for (const node of nodes) {
      this.add(node);
    }
  }

  update(node: AnyNodeData) {
    const existing = this.nodesById[node.id];
    if (!existing && !this.isPartial) throw new Error(`node [id=${node.id}] does not exist`);

    // remove/re-add to update with parent if needed, otherwise just update in place
    if (existing?.parentPtr?.id != node.parentPtr?.id) {
      if (existing != null) this.remove(existing);
      this.add(node);
    } else {
      this.nodesById[node.id] = node;
      if ("ck" in node) this.nodesByCk[node.ck] = node.id;
    }

    this.notify(node);
  }

  remove(node: AnyNodeData) {
    delete this.nodesById[node.id];
    if ("ck" in node) delete this.nodesByCk[node.ck];

    // remove from parent/roots
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
    for (const metatype in this.nodesByParentIdAndType[node.id]) {
      for (const childId of this.nodesByParentIdAndType[node.id][metatype]) {
        this.remove(this.nodesById[childId]);
      }
    }

    this.notify(node);
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const id = "id" in key ? key.id : this.nodesByCk[key.ck!];
    if (!id) return null;
    return (this.nodesById[id] ?? null) as NodeTypeMapping[T] | null;
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    const childrenIds = this.nodesByParentIdAndType[parent.id!]?.[metatype];
    if (!childrenIds) return [];
    const children = childrenIds.map((id) => this.nodesById[id]) as NodeTypeMapping[T][];
    defaultSort(metatype, children);
    return children;
  }

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): GraphRef<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<NodeKey<T> | null>;
    const unsub: () => void = () => keyRef.value == null || this.unsubscribe(keyRef.value, trigger);
    const get = () => (keyRef.value != null ? this.get(keyRef.value as NodeKey<T>) : null);
    const { ref, trigger } = manualGraphRef(get, unsub);
    watch(
      keyRef,
      (newKey, oldKey) => {
        if (newKey != oldKey) {
          // update subscription
          if (oldKey) this.unsubscribe(oldKey, trigger);
          if (newKey) this.subscribe(newKey, trigger);
        }
        trigger();
      },
      { immediate: true },
    );
    onUnmountedIfComponent(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | null>,
    metatype: T,
  ): GraphRef<NodeTypeMapping[T][]> {
    // TODO :Performance: trigger getChildrenRef more selectively
    // (discriminate parent update, individual node updates, ...)
    const parentRef = toRef(parent);
    const subs: Array<() => void> = [];
    const unsub = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    const get: () => NodeTypeMapping[T][] = () => {
      unsub();
      if (!parentRef.value) return [];
      const children = this.getChildren(parentRef.value, metatype);
      children.forEach((child) => this.subscribe(child, trigger));
      subs.push(this.subscribeChildren(parentRef.value, metatype, trigger));
      return children;
    };
    const { ref, trigger } = manualGraphRef(get, unsub);
    watch(parentRef, trigger, { immediate: true });
    onUnmountedIfComponent(unsub);
    return ref;
  }
}

/**
 * A graph composed of multiple (potentially overlapping subgraphs).
 * Nodes are merged from the layers in order, with later layers taking precedence.
 */
export class LayerNodeGraph extends ReactiveNodeGraphMixin implements ReadNodeGraph {
  public readonly layers: Ref<ReadNodeGraph[]>;

  constructor(layers: ReadNodeGraph[]) {
    super();
    this.layers = shallowRef(layers);
  }

  get isPartial(): boolean {
    return this.layers.value[0]?.isPartial ?? false;
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
      const children = layer.getChildren(parent, metatype);
      for (const child of children) {
        if (!mergedChildrenById[child.id]) {
          mergedChildrenById[child.id] = child;
        } else {
          mergedChildrenById[child.id] = mergeNode(mergedChildrenById[child.id], child);
        }
      }
    }
    const children = Object.values(mergedChildrenById);
    defaultSort(metatype, children);
    return children;
  }

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): GraphRef<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<NodeKey<T> | null>;
    const subs: Array<() => void> = [];
    const unsub = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    const get: () => NodeTypeMapping[T] | null = () => {
      unsub();
      if (!keyRef.value) return null;
      let mergedNode: NodeTypeMapping[T] | null = null;
      for (const layer of this.layers.value) {
        const node = layer.get<T>(keyRef.value);
        subs.push(layer.subscribe(keyRef.value, trigger));
        if (node) {
          if (!mergedNode) mergedNode = node;
          else mergedNode = mergeNode(mergedNode, node);
        }
      }
      return mergedNode;
    };
    const { ref, trigger } = manualGraphRef(get, unsub);
    watch(keyRef, trigger);
    onUnmountedIfComponent(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | null>,
    metatype: T,
  ): GraphRef<NodeTypeMapping[T][]> {
    const parentRef = toRef(parent);
    const subs: Array<() => void> = [];
    const unsub = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    const get: () => NodeTypeMapping[T][] = () => {
      unsub();
      if (!parentRef.value) return [];
      const mergedChildrenById: { [id: string]: NodeTypeMapping[T] } = {};
      for (const layer of this.layers.value) {
        subs.push(layer.subscribeChildren(parentRef.value, metatype, trigger));
        const children = layer.getChildren(parentRef.value, metatype);
        children.forEach((child) => {
          if (!mergedChildrenById[child.id]) {
            mergedChildrenById[child.id] = child;
          } else {
            mergedChildrenById[child.id] = mergeNode(mergedChildrenById[child.id], child);
          }
          subs.push(layer.subscribe(child, trigger));
        });
      }
      const children = Object.values(mergedChildrenById);
      defaultSort(metatype, children);
      return children;
    };
    const { ref, trigger } = manualGraphRef(get, unsub);
    watch(parentRef, trigger);
    onUnmountedIfComponent(unsub);
    return ref;
  }
}

/**
 * A 'view' of a graph with some nodes filtered out.
 */
export class FilterNodeGraph extends ReactiveNodeGraphMixin implements ReadNodeGraph {
  public readonly graph: ReadNodeGraph;
  public readonly includeHidden: boolean;

  constructor(graph: ReadNodeGraph, includeHidden: boolean) {
    super();
    this.graph = graph;
    this.includeHidden = includeHidden;
  }

  get scope(): GraphScope {
    return this.graph.scope;
  }

  get isPartial(): boolean {
    return this.graph.isPartial;
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

  getRef<T extends NodeType>(node: MaybeRef<NodeKey<T> | null>): GraphRef<NodeTypeMapping[T] | null> {
    const ref = this.graph.getRef(node);
    if (this.includeHidden) return ref;
    else
      return computedGraphRef(
        () => (ref.value && !ref.value.deletedAt && !ref.value.archivedAt ? ref.value : null),
        ref.stop,
      );
  }

  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | null>,
    metatype: T,
  ): GraphRef<NodeTypeMapping[T][]> {
    const ref = this.graph.getChildrenRef(parent, metatype);
    if (this.includeHidden) return ref;
    else return computedGraphRef(() => ref.value.filter((n) => !n.deletedAt && !n.archivedAt), ref.stop);
  }
}

export function mergeNode<T extends NodeType>(
  base: NodeTypeMapping[T],
  partial: NodeTypeMapping[T],
): NodeTypeMapping[T] {
  if (partial.setProperties.length > 0) {
    const merged: NodeTypeMapping[T] = { ...base };
    const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[base.metatype]!;
    for (const propId of partial.setProperties) {
      if (propId == allProperties.setProperties) {
        // merge setProperties
        merged.setProperties = [...merged.setProperties];
        partial.setProperties
          .filter((propId) => !merged.setProperties.includes(propId))
          .forEach((propId) => merged.setProperties.push(propId));
      } else {
        // overwrite property
        const propName = allProperties[propId];
        (merged as any)[propName] = (partial as any)[propName];
      }
    }
    return merged;
  } else {
    return { ...base, ...partial };
  }
}

export function nodeReference<T extends NodeType>(nodeType: T, id: string): NodeReferenceData {
  return { metatype: BenchType.NODE_REFERENCE, type: nodeType, id };
}

export function toNodeReference(node: null): null;
export function toNodeReference(node: AnyNodeData): NodeReferenceData;
export function toNodeReference(node: AnyNodeData | null): NodeReferenceData | null {
  if (!node) return null;
  const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const reference: NodeReferenceData = {
    metatype: BenchType.NODE_REFERENCE,
    type: node.metatype as unknown as NodeType,
    id: node.id,
  };
  if ("bench" in allProperties && node.parentPtr) {
    reference.benchId = node.parentPtr.benchId;
  }
  if ("ck" in allProperties) {
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

export function toNodeReferenceRef(node: MaybeRef<AnyNodeData | null>): Ref<NodeReferenceData | null> {
  const nodeRef = toRef(node) as Ref<AnyNodeData | null>;
  return computed(() => toNodeReference(nodeRef.value!)); // TODO :Cleanup: shouldn't have to ! to type check here?
}

export function patchReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultStruct(StructType.READ_OPTIONS),
    ...options,
  };
}

const graphsByScopeKey: Ref<{ [scopeKey: string]: NodeGraph }> = shallowRef({});

function getScopeKey(scope: GraphScope): string {
  return JSON.stringify(scope);
}

function getGraph(scope: GraphScope) {
  const key = getScopeKey(scope);
  if (!graphsByScopeKey.value[key]) {
    graphsByScopeKey.value[key] = new NodeGraph({ scope });
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

/**
 * Gets the given nodes from the relevant subgraph, fetching as needed.
 * If watch, will also ensure that edits for the given nodes are watched.
 */
export function getNodesRef<T extends NodeType>(
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
  let graph: NodeGraph | null = null;

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
      graph.extend(...fetched.nodes.map(unwrapSomeNode));
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

export function getChildrenRef<T extends NodeType>(
  parent: MaybeRef<AnyNodeData | null>,
  metatype: T,
): {
  graph: ReadNodeGraph;
  children: Ref<NodeTypeMapping[T][]>;
  error: Ref<OperationError | null>;
} {
  const parentRef = toRef(parent);
  const { graph, error } = getNodesRef(
    computed(() => ({
      roots: parentRef.value != null ? [toNodeReference(parentRef.value)!] : [],
      options: { descendantTypes: [metatype] },
    })),
  );
  const children = graph.getChildrenRef(toNodeReferenceRef(parentRef), metatype);
  return { graph, children, error };
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching as needed.
 * If watch, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function searchNodesRef<T extends NodeType>(
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
export function aggregateNodesRef(
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

export function editNodes(): { graph: ReadNodeGraph; transaction: Transaction } {
  throw new Error("not yet implemented");
}
