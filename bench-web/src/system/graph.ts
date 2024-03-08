import { getHostClient, supervisor, type Operation } from "@/proto/services";
import {
  AggregationData,
  BenchType,
  ExpressionData,
  GraphScope,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  ReadOptionsData,
  StructType,
  WatchEditsRequest,
  WatchEditsResponse,
  type AnyNodeData,
  type AnyPropertyType,
  type IGraphIOClient,
  type NodeType,
  type NodeTypeMapping,
} from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import type { AccessQuery } from "@/system/access";
import { BASED_NODE_TYPES, defaultSort, getBaseFromNode } from "@/system/lang";
import { TransactionBuffer, useGraphContext } from "@/system/transaction";
import { computedSubRef, manualSubRef, onUnmountedIfComponent, type SubRef } from "@/utils/ref";
import type { Transaction } from "@sentry/vue";
import { computed, ref, shallowRef, toRef, watch, type MaybeRef, type Ref, isRef } from "vue";

/** A NodeReference but with proper typing */
export type NodeKey<T extends NodeType> = Omit<NodeReferenceData, "metatype" | "type"> & { type?: T };

/** A node graph with change subscriptions */
export type ObservableNodeGraph = {
  /** Subs */
  subscribe(key: { id?: string; ck?: string }, callback: () => void): () => void;
  subscribeChildren<T extends NodeType>(
    parent: { id?: string; ck?: string },
    metatype: T,
    callback: () => void,
  ): () => void;
};

/** A node graph with read methods */
export type ReadNodeGraph = {
  /** The scope contained in this graph */
  get scope(): GraphScope;
  /** Whether this graph is partial */
  readonly isPartial: boolean;
  /** All the nodes in this graph */
  get nodes(): AnyNodeData[];
  /** Number of nodes in this graph */
  get size(): number;
  /** Gets the current node with that key (not reactive) */
  get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  /** Gets the children of the given parent with the given metatype (not reactive) */
  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][];
  /** Gets a reactive reference to the current node with that key */
  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): SubRef<NodeTypeMapping[T] | null>;
  /** Gets a reactive reference to the children of the given parent with the given metatype */
  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): SubRef<NodeTypeMapping[T][]>;
};

export type ObservableReadNodeGraph = ReadNodeGraph & ObservableNodeGraph;

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
 * Helper mixin for managing reactivity in a graph.
 */
class ObservableNodeGraphMixin implements ObservableNodeGraph {
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
export class NodeGraph extends ObservableNodeGraphMixin implements ReadNodeGraph, WriteNodeGraph {
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

  get nodes(): AnyNodeData[] {
    return Object.values(this.nodesById);
  }

  get size(): number {
    return Object.keys(this.nodesById).length;
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

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): SubRef<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<NodeKey<T> | null>;
    let sub: (() => void) | null = null;
    const unsub: () => void = () => (sub != null ? (sub(), (sub = null)) : null);
    const get = () => (keyRef.value != null ? this.get(keyRef.value as NodeKey<T>) : null);
    const { ref, trigger } = manualSubRef(get, unsub);
    watch(
      keyRef,
      (newKey, oldKey) => {
        if (newKey != oldKey) {
          // update subscription
          if (oldKey) unsub();
          if (newKey) sub = this.subscribe(newKey, trigger);
        }
        trigger();
      },
      { immediate: true },
    );
    onUnmountedIfComponent(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): SubRef<NodeTypeMapping[T][]> {
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
    const { ref, trigger } = manualSubRef(get, unsub);
    watch(parentRef, trigger, { immediate: true });
    onUnmountedIfComponent(unsub);
    return ref;
  }
}

/**
 * A graph composed of multiple (potentially overlapping subgraphs).
 * Nodes are merged from the layers in order, with later layers taking precedence.
 */
export class LayerNodeGraph implements ReadNodeGraph {
  // TODO :Performance: LayerNodeGraph.layers should be scoped
  //  (so we only need to acquire refs from layers with the requested scope)
  public readonly layers: Ref<ObservableReadNodeGraph[]>;

  constructor(layers: MaybeRef<ObservableReadNodeGraph[]>) {
    this.layers = !isRef(layers) ? shallowRef(layers) : layers;
  }

  get isPartial(): boolean {
    return this.layers.value[0]?.isPartial ?? false;
  }

  get nodes(): AnyNodeData[] {
    const nodesById: { [id: string]: AnyNodeData } = {};
    for (const layer of this.layers.value) {
      for (const node of layer.nodes) {
        if (!nodesById[node.id]) nodesById[node.id] = node;
        else nodesById[node.id] = mergeNode(nodesById[node.id], node);
      }
    }
    return Object.values(nodesById);
  }

  get size(): number {
    return this.nodes.length;
  }

  resetLayers() {
    this.layers.value = [];
  }

  addLayer(layer: ObservableReadNodeGraph) {
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

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): SubRef<NodeTypeMapping[T] | null> {
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
    const { ref, trigger } = manualSubRef(get, unsub);
    watch([keyRef, this.layers], trigger);
    onUnmountedIfComponent(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): SubRef<NodeTypeMapping[T][]> {
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
    const { ref, trigger } = manualSubRef(get, unsub);
    watch([parentRef, this.layers], trigger);
    onUnmountedIfComponent(unsub);
    return ref;
  }
}

/**
 * A proxy to a single graph (like a LayerNodeGraph with a single layer).
 */
export class ProxyNodeGraph implements ReadNodeGraph {
  public readonly graph: Ref<ObservableReadNodeGraph | null>;

  constructor(graph: ObservableReadNodeGraph | null) {
    this.graph = shallowRef(graph);
  }

  get scope(): GraphScope {
    return this.graph.value?.scope ?? {};
  }

  get isPartial(): boolean {
    return this.graph.value?.isPartial ?? false;
  }

  get nodes(): AnyNodeData[] {
    return this.graph.value?.nodes ?? [];
  }

  get size(): number {
    return this.graph.value?.size ?? 0;
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    return this.graph.value?.get(key) ?? null;
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    return this.graph.value?.getChildren(parent, metatype) ?? [];
  }

  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): SubRef<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<NodeKey<T> | null>;
    let sub: (() => void) | null = null;
    const unsub = () => sub != null && sub();
    const get = () => {
      unsub();
      if (keyRef.value && this.graph.value) {
        sub = this.graph.value.subscribe(keyRef.value, trigger);
        return this.graph.value.get(keyRef.value);
      } else {
        return null;
      }
    };
    const { ref, trigger } = manualSubRef(get, unsub);
    watch(keyRef, trigger);
    onUnmountedIfComponent(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): SubRef<NodeTypeMapping[T][]> {
    const parentRef = toRef(parent);
    const subs: Array<() => void> = [];
    const unsub = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    const get: () => NodeTypeMapping[T][] = () => {
      unsub();
      if (parentRef.value && this.graph.value) {
        subs.push(this.graph.value.subscribeChildren(parentRef.value, metatype, trigger));
        const children = this.graph.value.getChildren(parentRef.value, metatype);
        children.forEach((child) => {
          subs.push(this.graph.value!.subscribe(child, trigger));
        });
        return children;
      } else {
        return [];
      }
    };
    const { ref, trigger } = manualSubRef(get, unsub);
    watch(parentRef, trigger);
    onUnmountedIfComponent(unsub);
    return ref;
  }
}

/**
 * A 'view' of a graph with some nodes filtered out.
 */
export class FilterNodeGraph implements ReadNodeGraph {
  public readonly graph: ReadNodeGraph;
  public readonly includeHidden: Ref<boolean>;

  constructor(graph: ReadNodeGraph, includeHidden: boolean) {
    this.graph = graph;
    this.includeHidden = ref(includeHidden);
  }

  get scope(): GraphScope {
    return this.graph.scope;
  }

  get isPartial(): boolean {
    return this.graph.isPartial;
  }

  get nodes(): AnyNodeData[] {
    if (this.includeHidden.value) return this.graph.nodes;
    else return this.graph.nodes.filter((n) => !n.deletedAt && !n.archivedAt);
  }

  get size(): number {
    return this.nodes.length;
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const node = this.graph.get(key);
    if (!this.includeHidden.value && (!node || node.deletedAt || node.archivedAt)) return null;
    else return node;
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    const children = this.graph.getChildren(parent, metatype);
    if (!this.includeHidden.value) return children.filter((n) => !n.deletedAt && !n.archivedAt);
    else return children;
  }

  getRef<T extends NodeType>(node: MaybeRef<NodeKey<T> | null>): SubRef<NodeTypeMapping[T] | null> {
    const ref = this.graph.getRef(node);
    return computedSubRef(
      () => (ref.value && (this.includeHidden || (!ref.value.deletedAt && !ref.value.archivedAt)) ? ref.value : null),
      ref.stop,
    );
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): SubRef<NodeTypeMapping[T][]> {
    const ref = this.graph.getChildrenRef(parent, metatype);
    return computedSubRef(() => {
      if (this.includeHidden) return ref.value;
      else return ref.value.filter((n) => !n.deletedAt && !n.archivedAt);
    }, ref.stop);
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

function patchReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultStruct(StructType.READ_OPTIONS),
    ...options,
  };
}

function deriveScope(roots: NodeReferenceData[]): GraphScope {
  const scope: GraphScope = {};
  for (const root of roots) {
    if (root.benchId) {
      if (scope.benchId && scope.benchId != root.benchId) {
        throw new Error(`cannot derive scope from multiple benches: ${scope.benchId} and ${root.benchId}`);
      }
      scope.benchId = root.benchId;
    }
  }
  return scope;
}

export function getScopeKey(scope: GraphScope): string {
  return JSON.stringify(scope);
}

/**
 * A connection to a graph client for an overlapping set of read operations.
 */
export type GraphConnection = {
  scope: GraphScope;
  graph: ReadNodeGraph;
  access: AccessQuery;
  tx: TransactionBuffer; // shared per host
  client: IGraphIOClient;
  options: ReadOptionsData;
};

async function getGraphClient(scope: GraphScope): Promise<IGraphIOClient> {
  if (scope.benchId) {
    return await getHostClient({ id: scope.benchId });
  } else {
    return supervisor;
  }
}

/**
 * Gets the current graph for the given scope.
 */
export function getGraph(): { graph: ReadNodeGraph; connection: GraphConnection } {
  throw new Error("not yet implemented");
}

/**
 * Gets the given nodes from the relevant subgraph, fetching as needed.
 * If watch, will also ensure that edits for the given nodes are watched.
 */
export function getNodesRef<T extends NodeType>(
  request: MaybeRef<{
    scope?: GraphScope; // else scope is inferred from refs & context
    roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
    options?: Partial<ReadOptionsData>;
    watch?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
} {
  // TODO :Architecture: where should optimistic & overlay graphs be composed?
  const requestRef = toRef(request);
  const graphLayers = shallowRef<ObservableReadNodeGraph[]>([]);
  const graph = new LayerNodeGraph(graphLayers);
  const context = useGraphContext();

  // const roots

  return { graph } as any /* nocheckin */;
}

/**
 * Gets a single node from the relevant subgraph, fetching as needed.
 * If watch, will also ensure that edits for the given node are watched.
 */
export function getNodeRef<T extends NodeType>(
  request: MaybeRef<{
    scope?: GraphScope; // else scope is inferred from refs & context
    root: Omit<NodeReferenceData, "type"> & { type: T };
    options?: Partial<ReadOptionsData>;
    watch?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  root: SubRef<NodeTypeMapping[T] | null>;
} {
  throw new Error("not yet implemented");
}

/**
 * Gets the children of the given parent in the current scope. No fetch.
 */
export function getChildrenRef<T extends NodeType>(
  parent: MaybeRef<NodeReferenceData | null>,
  metatype: T,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  children: SubRef<NodeTypeMapping[T][]>;
} {
  const parentRef = toRef(parent);
  const { graph, connection } = getNodesRef(
    computed(() => ({
      roots: parentRef.value != null ? [parentRef.value!] : [],
      options: { descendantTypes: [metatype] },
    })),
  );
  const children = graph.getChildrenRef(parentRef, metatype);
  return { graph, connection, children };
}

/**
 * Searches for nodes of the given type in the relevant subgraph. Always fetches.
 * If watch, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function searchNodesRef<T extends NodeType>(
  request: MaybeRef<{
    scope?: GraphScope; // else scope is inferred from refs & context
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
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
  page: Ref<{ cursors: string[]; startCursor: string; size: number; total?: number }>;
} {
  throw new Error("not yet implemented");
}

/**
 * Aggregates nodes of the given type in the relevant subgraph. Always fetches.
 * TODO :Feature: watch aggregation
 */
export function aggregateNodesRef(
  aggregate: MaybeRef<{
    scope?: GraphScope; // else scope is inferred from refs & context
    nodeType: NodeType;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    aggregation: ExpressionData;
    enabled?: boolean;
  }>,
): { aggregation: SubRef<AggregationData> } {
  throw new Error("not yet implemented");
}

export function editNodes(): { graph: ReadNodeGraph; transaction: Transaction } {
  throw new Error("not yet implemented");
}
