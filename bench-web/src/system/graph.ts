import {
  GraphScope,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  type AnyNodeData,
  type AnyPropertyType,
  type NodeType,
  type NodeTypeMapping,
} from "@/proto/wire";
import { describeNode } from "@/proto/wiring";
import { defaultSort } from "@/system/lang";
import { manualSubRef, type SubRef } from "@/utils/ref";
import { tryOnBeforeUnmount } from "@vueuse/core";
import { isRef, ref, shallowRef, toRef, watch, type MaybeRef, type Ref, type ShallowRef } from "vue";

/** A NodeReference but with proper typing */
export type NodeKey<T extends NodeType> = Omit<NodeReferenceData, "metatype" | "type"> & { type?: T };

/** A node graph with read methods */
export interface ReadNodeGraph {
  /** The scope contained in this graph */
  get scope(): GraphScope;
  /** Whether this graph is partial */
  readonly isPartial: boolean;
  /** All the nodes in this graph */
  get nodes(): AnyNodeData[];
  /** Number of nodes in this graph */
  get size(): number;
  /** The (relative) roots (nodes without parents in graph) */
  get roots(): AnyNodeData[];
  /** Gets the current node with that key if present (not reactive) */
  get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  /** Gets the current node with that given key (error if not found) */
  getOrFail<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T];
  /** Gets the children of the given parent with the given metatype (not reactive) */
  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][];
  /** Subscribe to any change in the given key */
  subscribe(key: { id?: string; ck?: string }, callback: () => void): () => void;
  /** Subscribe to any change in the given children */
  subscribeChildren<T extends NodeType>(
    parent: { id?: string; ck?: string },
    metatype: T,
    callback: () => void,
  ): () => void;

  /**
   * General helpers
   */
  /** Gets the current node with the key if the key is given */
  getMaybe<T extends NodeType>(key: NodeKey<T> | undefined | null): NodeTypeMapping[T] | null;
  /** * Gets all ancestors of the given node with the given or any metatypes. */
  getAncestors(node: NodeKey<any>, metatypes?: NodeType[]): AnyNodeData[];
  /**
   * Gets all descendants of the given parent with the given metatypes, matching a certain filter.
   * The filter must depend on only the given node.
   */
  getDescendants(parent: NodeKey<any>, metatypes: NodeType[], filter?: (node: AnyNodeData) => boolean): AnyNodeData[];

  //
  // Observable helpers
  //

  /** Gets a reactive reference to the current node with that key */
  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | undefined | null>): SubRef<NodeTypeMapping[T] | null>;
  /** Gets a reactive reference to many nodes with the given keys (missing nodes excluded) */
  getManyRef<T extends NodeType>(keys: MaybeRef<NodeKey<T>[] | undefined | null>): SubRef<NodeTypeMapping[T][]>;
  /** Gets a reactive reference to the children of the given parent with the given metatype */
  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | undefined | null>,
    metatype: T,
  ): SubRef<NodeTypeMapping[T][]>;
}

/** A node graph with write methods */
export interface WriteNodeGraph {
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
  /** Clears all nodes in this graph */
  clear(): void;
}

/**
 * Helper mixin for managing in a graph.
 */
abstract class BaseNodeGraphMixin implements Omit<ReadNodeGraph, "scope" | "isPartial" | "size"> {
  abstract nodes: AnyNodeData[];
  abstract get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  abstract getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][];
  abstract subscribe(key: { id?: string | undefined; ck?: string | undefined }, callback: () => void): () => void;
  abstract subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: () => void,
  ): () => void;

  get roots() {
    return this.nodes.filter((n) => n.parentPtr == null || this.get(n.parentPtr) == null);
  }

  describeSelf(): string {
    return `${this.constructor.name}(${this.roots.map(describeNode)})`;
  }

  getOrFail<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] {
    const node = this.get(key);
    if (node == null) throw new Error(`node ${describeNode(key)} not found in ${this.describeSelf()}`);
    return node;
  }

  getMaybe<T extends NodeType>(key: NodeKey<T> | undefined | null): NodeTypeMapping[T] | null {
    return key ? this.get(key) : null;
  }

  getAncestors(node: NodeKey<any>, metatypes?: NodeType[] | undefined): AnyNodeData[] {
    const ancestors: AnyNodeData[] = [];
    let parent = this.get(node)?.parentPtr;
    while (parent != null) {
      const parentNode = this.get(parent);
      if (parentNode == null) break;
      if (metatypes == null || metatypes.includes(parentNode.metatype as unknown as NodeType)) {
        ancestors.push(parentNode);
      }
      parent = parentNode.parentPtr;
    }
    return ancestors;
  }

  getDescendants(parent: NodeKey<any>, metatypes: NodeType[], filter?: (node: AnyNodeData) => boolean): AnyNodeData[] {
    const descendants: AnyNodeData[] = [];
    const children = [];
    for (const metatype of metatypes) {
      children.push(...this.getChildren(parent, metatype));
    }

    for (const child of children) {
      if (filter == null || filter(child)) {
        descendants.push(child);
        descendants.push(...this.getDescendants(child, metatypes, filter));
      }
    }
    return descendants;
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
          if (oldKey) unsub();
          if (newKey) sub = this.subscribe(newKey, trigger);
        }
        trigger();
      },
      { immediate: true },
    );
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  getManyRef<T extends NodeType>(keys: MaybeRef<NodeKey<T>[] | null | undefined>): SubRef<NodeTypeMapping[T][]> {
    const keysRef = toRef(keys) as Ref<NodeKey<T>[] | null | undefined>;
    const subs: (() => void)[] = [];
    const unsub: () => void = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    const get = () =>
      keysRef.value != null
        ? (keysRef.value.map((key) => this.get(key)).filter((n) => n != null) as NodeTypeMapping[T][])
        : [];
    const { ref, trigger } = manualSubRef(get, unsub);
    watch(
      keysRef,
      (newKeys, oldKeys) => {
        if (newKeys != oldKeys) {
          if (oldKeys) unsub();
          if (newKeys) newKeys.filter((key) => key != null).forEach((key) => subs.push(this.subscribe(key, trigger)));
        }
        trigger();
      },
      { immediate: true },
    );
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(parent: MaybeRef<NodeKey<any> | null>, metatype: T): SubRef<NodeTypeMapping[T][]> {
    // TODO :Performance: trigger getChildrenRef more selectively
    // (discriminate parent update, individual node updates, ...)
    const parentRef = toRef(parent);
    let sub: (() => void) | null = null;
    const unsub: () => void = () => (sub != null ? (sub(), (sub = null)) : null);
    const get: () => NodeTypeMapping[T][] = () =>
      parentRef.value != null ? this.getChildren(parentRef.value, metatype) : [];
    const { ref, trigger } = manualSubRef(get, unsub);
    watch(
      parentRef,
      (newParent, oldParent) => {
        if (newParent != oldParent) {
          unsub();
          if (newParent) sub = this.subscribeChildren(newParent, metatype, trigger);
        }
        trigger();
      },
      { immediate: true },
    );
    tryOnBeforeUnmount(unsub);
    return ref;
  }
}

/**
 * Core in-memory node graph without regard for hidden nodes or multi-graphs (deleted, archived, etc.).
 * If 'isPartial', we don't try to maintain local consistency (as this is likely an overlay in a layered graph).
 */
export class NodeGraph extends BaseNodeGraphMixin implements ReadNodeGraph, WriteNodeGraph {
  public readonly scope: GraphScope = {};
  public readonly isPartial: boolean = false;
  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByCk: { [ck: string]: string } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private rootsIds: string[] = [];
  private subsById: { [id: string]: Array<() => void> } = {};
  private subsByCk: { [ck: string]: Array<() => void> } = {};
  private subsByParentIdAndType: { [parentId: string]: { [type: string]: Array<() => void> } } = {};

  constructor(options: { scope?: GraphScope; isPartial?: boolean } = { scope: {}, isPartial: false }) {
    super();
    this.scope = options.scope ?? {};
    this.isPartial = options.isPartial ?? false;
  }

  add(node: AnyNodeData) {
    if (!node.id) throw new Error("node must have an id");
    if (this.nodesById[node.id])
      throw new Error(`node ${describeNode(node)} id already exists in ${this.describeSelf()}`);
    this.nodesById[node.id] = node;
    if ("ck" in node) {
      if (this.nodesByCk[node.ck])
        throw new Error(`node ${describeNode(node)} ck already exists in ${this.describeSelf()}`);
      this.nodesByCk[node.ck] = node.id;
    }

    // add to parent/roots
    this._addToParent(node);

    this.notify(node);
  }

  extend(...nodes: AnyNodeData[]) {
    for (const node of nodes) {
      this.add(node);
    }
  }

  update(node: AnyNodeData) {
    if (!node.id) throw new Error(`node must have an id: ${describeNode(node)}`);
    const existing = this.nodesById[node.id];
    if (!existing && !this.isPartial) throw new Error(`node ${describeNode(node)} not found in ${this.describeSelf()}`);

    if (existing != null && existing?.parentPtr?.id != node.parentPtr?.id) {
      // move
      if (existing?.parentPtr != null) this._removeFromParent(existing!);
      if (node.parentPtr != null) this._addToParent(node);
      this.nodesById[node.id] = node;
      if (existing) this.notify(existing);
      this.notify(node);
    } else {
      // simple in place update
      this.nodesById[node.id] = node;
      if ("ck" in node) this.nodesByCk[node.ck] = node.id;
      this.notify(node);
    }
  }

  remove(node: AnyNodeData) {
    if (!node.id) throw new Error(`node must have an id: ${describeNode(node)}`);
    delete this.nodesById[node.id];
    if ("ck" in node) delete this.nodesByCk[node.ck];

    // remove from parent/roots
    this._removeFromParent(node);

    // remove any children (recursively)
    for (const metatype in this.nodesByParentIdAndType[node.id]) {
      for (const childId of this.nodesByParentIdAndType[node.id][metatype]) {
        this.remove(this.nodesById[childId]);
      }
    }

    this.notify(node);
  }

  clear() {
    // remove all nodes
    this.nodesById = {};
    this.nodesByCk = {};
    this.nodesByParentIdAndType = {};

    // notify (and clear) all subs
    for (const id in this.subsById) {
      this.subsById[id].forEach((sub) => sub());
    }
    for (const ck in this.subsByCk) {
      this.subsByCk[ck].forEach((sub) => sub());
    }
    for (const parentId in this.subsByParentIdAndType) {
      for (const metatype in this.subsByParentIdAndType[parentId]) {
        this.subsByParentIdAndType[parentId][metatype].forEach((sub) => sub());
      }
    }
    this.subsById = {};
    this.subsByCk = {};
    this.subsByParentIdAndType = {};
  }

  private _addToParent(node: AnyNodeData) {
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      if (!this.nodesById[parentId] && !this.isPartial) {
        throw new Error(
          `parent ${describeNode(node.parentPtr)} not found in ${this.describeSelf()} for node ${describeNode(node)}`,
        );
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

  private _removeFromParent(node: AnyNodeData) {
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      const nodeIdx = this.nodesByParentIdAndType[parentId]?.[node.metatype]?.findIndex((n) => n == node.id);
      if (nodeIdx == null && this.isPartial) return;
      else if (nodeIdx == -1)
        throw new Error(
          `node ${describeNode(node)} not found in parent ${describeNode(node.parentPtr)} in ${this.describeSelf()}`,
        );
      this.nodesByParentIdAndType[parentId][node.metatype].splice(nodeIdx, 1);
    } else {
      const rootIdx = this.rootsIds.findIndex((n) => n == node.id);
      if (rootIdx == -1) throw new Error(`node ${describeNode(node)} not found in roots of ${this.describeSelf()}`);
      this.rootsIds.splice(rootIdx, 1);
    }
  }

  get nodes(): AnyNodeData[] {
    return Object.values(this.nodesById);
  }

  get size(): number {
    return Object.keys(this.nodesById).length;
  }

  get roots(): AnyNodeData[] {
    return this.rootsIds.map((id) => this.nodesById[id]);
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const id = "id" in key ? key.id : this.nodesByCk[key.ck!];
    if (!id) return null;
    return (this.nodesById[id] ?? null) as NodeTypeMapping[T] | null;
  }

  getMany<T extends NodeType>(keys: NodeKey<T>[]): NodeTypeMapping[T][] {
    return keys.map((key) => this.get(key)).filter((n) => n != null) as NodeTypeMapping[T][];
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    const childrenIds = this.nodesByParentIdAndType[parent.id!]?.[metatype];
    if (!childrenIds) return [];
    const children = childrenIds.map((id) => this.nodesById[id]) as NodeTypeMapping[T][];
    defaultSort(metatype, children);
    return children;
  }

  subscribe(key: { id?: string; ck?: string }, callback: () => void): () => void {
    if (key.id) {
      if (!this.subsById[key.id]) this.subsById[key.id] = [];
      this.subsById[key.id].push(callback);
    }
    if (key.ck) {
      if (!this.subsByCk[key.ck]) this.subsByCk[key.ck] = [];
      this.subsByCk[key.ck].push(callback);
    }
    return () => {
      if (key.id) {
        if (this.subsById[key.id]) this.subsById[key.id].splice(this.subsById[key.id].indexOf(callback), 1);
      }
      if (key.ck) {
        if (this.subsByCk[key.ck]) this.subsByCk[key.ck].splice(this.subsByCk[key.ck].indexOf(callback), 1);
      }
    };
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
    return () => {
      if (parent.id == null) throw new Error("parent must have an id");
      if (this.subsByParentIdAndType[parent.id] && this.subsByParentIdAndType[parent.id][metatype]) {
        this.subsByParentIdAndType[parent.id][metatype].splice(
          this.subsByParentIdAndType[parent.id][metatype].indexOf(callback),
          1,
        );
        // cleanup
        if (this.subsByParentIdAndType[parent.id][metatype].length == 0)
          delete this.subsByParentIdAndType[parent.id][metatype];
        if (Object.keys(this.subsByParentIdAndType[parent.id]).length == 0)
          delete this.subsByParentIdAndType[parent.id];
      }
    };
  }

  notify(node: AnyNodeData) {
    if (this.subsById[node.id]) {
      this.subsById[node.id].forEach((sub) => sub());
    }
    if ("ck" in node && this.subsByCk[node.ck]) {
      this.subsByCk[node.ck].forEach((sub) => sub());
    }
    if (node.parentPtr?.id && this.subsByParentIdAndType[node.parentPtr.id]) {
      const subs = this.subsByParentIdAndType[node.parentPtr.id][node.metatype];
      if (subs) subs.forEach((sub) => sub());
    }
  }
}

/**
 * A graph composed of multiple (potentially overlapping subgraphs).
 * Nodes are merged from the layers in order, with later layers taking precedence.
 */
export class LayerNodeGraph extends BaseNodeGraphMixin implements ReadNodeGraph {
  // TODO :Performance: LayerNodeGraph.layers should be scoped
  //  (so we only need to acquire refs from layers with the requested scope)
  public readonly layers: ShallowRef<ReadNodeGraph[]>;

  constructor(layers: MaybeRef<ReadNodeGraph[]>) {
    super();
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

  subscribe(key: { id?: string | undefined; ck?: string | undefined }, callback: () => void): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    watch(
      this.layers,
      () => {
        callback();
        unsub();
        this.layers.value.forEach((layer) => subs.push(layer.subscribe(key, callback)));
      },
      { immediate: true, flush: "sync" },
    );
    return () => subs.forEach((sub) => sub());
  }

  subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: () => void,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => subs.forEach((sub) => sub(), subs.splice(0, subs.length));
    watch(
      this.layers,
      () => {
        callback();
        unsub();
        this.layers.value.forEach((layer) => subs.push(layer.subscribeChildren(parent, metatype, callback)));
      },
      { immediate: true, flush: "sync" },
    );
    return () => subs.forEach((sub) => sub());
  }
}

/**
 * A proxy to a single graph (like a LayerNodeGraph with a single layer).
 */
export class ProxyNodeGraph extends BaseNodeGraphMixin implements ReadNodeGraph {
  readonly _graph: ShallowRef<ReadNodeGraph | null>;

  constructor(graph: MaybeRef<ReadNodeGraph | null>) {
    super();
    this._graph = isRef(graph) ? graph : shallowRef(graph);
  }

  public get graph(): ReadNodeGraph | null {
    return this._graph.value;
  }

  public set graph(graph: ReadNodeGraph | null) {
    if (this._graph.value !== graph) this._graph.value = graph;
  }

  get scope(): GraphScope {
    return this._graph.value?.scope ?? {};
  }

  get isPartial(): boolean {
    return this._graph.value?.isPartial ?? false;
  }

  get nodes(): AnyNodeData[] {
    return this._graph.value?.nodes ?? [];
  }

  get size(): number {
    return this._graph.value?.size ?? 0;
  }

  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    return this._graph.value?.get(key) ?? null;
  }

  getChildren<T extends NodeType>(parent: NodeKey<any>, metatype: T): NodeTypeMapping[T][] {
    return this._graph.value?.getChildren(parent, metatype) ?? [];
  }

  subscribe(key: { id?: string | undefined; ck?: string | undefined }, callback: () => void): () => void {
    let sub: (() => void) | null;
    const unsub = () => (sub != null ? (sub(), (sub = null)) : null);
    watch(
      this._graph,
      () => {
        callback();
        unsub();
        sub = this._graph.value?.subscribe(key, callback) ?? null;
      },
      { immediate: true, flush: "sync" },
    );
    return unsub;
  }

  subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: () => void,
  ): () => void {
    let sub: (() => void) | null;
    const unsub = () => (sub != null ? (sub(), (sub = null)) : null);
    watch(
      this._graph,
      () => {
        callback();
        unsub();
        sub = this._graph.value?.subscribeChildren(parent, metatype, callback) ?? null;
      },
      { immediate: true, flush: "sync" },
    );
    return unsub;
  }
}

/**
 * A 'view' of a graph with some nodes filtered out.
 */
export class FilterNodeGraph extends BaseNodeGraphMixin implements ReadNodeGraph {
  public readonly graph: ReadNodeGraph;
  public readonly includeHidden: Ref<boolean>;

  constructor(graph: ReadNodeGraph, includeHidden: boolean) {
    super();
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

  subscribe(key: { id?: string | undefined; ck?: string | undefined }, callback: () => void): () => void {
    let sub: (() => void) | null = null;
    const unsub = () => (sub != null ? (sub(), (sub = null)) : null);
    watch(
      this.includeHidden,
      () => {
        callback();
        unsub();
        sub = this.graph.subscribe(key, callback);
      },
      { immediate: true, flush: "sync" },
    );
    return unsub;
  }

  subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: () => void,
  ): () => void {
    let sub: (() => void) | null = null;
    const unsub = () => (sub != null ? (sub(), (sub = null)) : null);
    watch(
      this.includeHidden,
      () => {
        callback();
        unsub();
        sub = this.graph.subscribeChildren(parent, metatype, callback);
      },
      { immediate: true, flush: "sync" },
    );
    return unsub;
  }
}

/**
 * Creates a new node with the explicitly set properties from the overlay superimposed on the base.
 * NOTE: this works specifically with Node.setProperties, not a TS Partial.
 */
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
