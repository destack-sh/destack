import {
  CHILD_NODE_TYPES,
  GraphScopeData,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeType,
  type AnyNodeData,
  type AnyPropertyType,
  type NodeTypeMapping,
} from "@/proto/wire";
import {
  EMPTY_SCOPE,
  describeNode,
  describeScope,
  isNodeRef,
  toNodeRef,
  type AnyNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { toCamelName } from "@/language/const";
import { computedValue, manualSubRef, watchValue, type SubRef } from "@/utils/ref";
import { tryOnBeforeUnmount } from "@vueuse/core";
import {
  isRef,
  shallowRef,
  toRef,
  triggerRef,
  watch,
  type MaybeRef,
  type Ref,
  type ShallowRef,
  type WatchSource,
} from "vue";
import { defaultSortNode } from "@/language/order";

/** A NodeReference but with proper typing */
export type NodeKey<T extends NodeType> = { type?: T; id?: string; ck?: string };

// NOTE :Performance!: should differentiate node update types for :NodeFiltering
//  (e.g. full, create/delete, move, update:[properties...], etc.)
export type NodeGraphCallback = () => void;
export type NodeSubscriptionOptions = {
  ignoreAncestors?: boolean;
};

/** A filter for nodes in a graph. Nodes pretend to not be in the graph when this predicate fails. */
export type NodeGraphFilter = {
  /** Hidden = archivedAt|deletedAt */
  includeHidden: boolean;
};
export const DEFAULT_NODE_FILTER = { includeHidden: false };
export const PASSTHROUGH_NODE_FILTER = { includeHidden: true };

/** A node graph with read methods */
export interface ReadNodeGraph {
  describeSelf(): string;

  /** The scope contained in this graph */
  get scope(): GraphScopeData;

  /** The node types contained in this graph */
  get nodeTypes(): NodeType[];

  /** Whether this graph is partial */
  readonly isOverlayOf: ReadNodeGraph | null;

  /** All the nodes in this graph */
  get nodes(): AnyNodeData[];

  /** Number of nodes in this graph */
  get size(): number;

  /** The (relative) roots (nodes without parents in graph) */
  get roots(): AnyNodeData[];

  /** Whether this graph contains the given node */
  has(node: NodeKey<any>): boolean;

  /** Gets the current node with that key if present (not reactive) */
  get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;

  /** Gets the children of the given parent with the given metatype (not reactive) */
  getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][];

  //
  // General helpers
  //

  /** Gets the current node with that given key (error if not found) */
  getOrError<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T];

  /** Gets the current node with the key if the key is given */
  getMaybe<T extends NodeType>(key: NodeKey<T> | undefined | null): NodeTypeMapping[T] | null;

  /**Gets all ancestors of the given node with the given or any metatypes. */
  getAncestors<T extends NodeType = NodeType>(
    node: NodeKey<any>,
    options?: { metatypes?: T[]; includeSelf?: boolean; root?: NodeKey<any> },
  ): NodeTypeMapping[T][];

  /** Gets all descendants of the given parent with the given metatypes, matching a certain filter. */
  getDescendants<T extends NodeType = NodeType>(
    node: NodeKey<any>,
    options?: { metatypes?: T[]; filter?: (node: NodeTypeMapping[T]) => boolean; includeSelf?: boolean },
  ): NodeTypeMapping[T][];

  //
  // Observable helpers
  //

  /** Subscribe to any change in the given key */
  subscribe(
    key: { id?: string; ck?: string },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void;

  /** Subscribe to any change in the given parent's children */
  subscribeChildren<T extends NodeType>(
    parent: { id?: string; ck?: string },
    metatype: T,
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void;

  /** Subscribes to any change in the given parent's ancestors */
  subscribeAncestors(
    node: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
  ): () => void;

  /** Gets a reactive reference to the current node with that key */
  getRef<T extends NodeType>(
    key: MaybeRef<NodeKey<T> | undefined | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T] | null>;

  /** Gets a reactive reference to many nodes with the given keys (missing nodes excluded) */
  getManyRef<T extends NodeType>(
    keys: MaybeRef<NodeKey<T>[] | undefined | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T][]>;

  /** Gets a reactive reference to many nodes with potentially unset keys (missing nodes included as null) */
  getManyMaybeRef<T extends NodeType>(
    keys: MaybeRef<(NodeKey<T> | undefined | null)[] | undefined | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<(NodeTypeMapping[T] | null)[]>;

  /** Gets a reactive reference to the children of the given parent with the given metatype */
  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | undefined | null>,
    metatype: T,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T][]>;

  /** Gets ancestors reactively */
  getAncestorsRef<T extends NodeType>(
    key: MaybeRef<NodeKey<any> | undefined | null>,
    options?: { metatypes?: T[]; includeSelf?: boolean; root?: MaybeRef<NodeKey<any> | undefined | null> },
  ): SubRef<NodeTypeMapping[T][]>;
}

/** A node graph with write methods */
export interface WriteNodeGraph {
  /** The scope contained in this graph */
  get scope(): GraphScopeData;
  /** Adds a node to the graph (error if exists) */
  add(node: AnyNodeData): void;
  /** Adds multiple nodes to the graph (error if exists) */
  extend(...nodes: AnyNodeData[]): void;
  /** Updates an existing node in the graph (error if does not exist) */
  update(node: AnyNodeData): void;
  /** Removes a node from the graph (error if does not exist) */
  remove(node: AnyNodeData): void;
  /** Removes all nodes in this graph */
  clear(): void;
}

/**
 * Helper mixin for managing in a graph.
 */
abstract class BaseNodeGraphMixin implements ReadNodeGraph {
  abstract scope: GraphScopeData;
  abstract nodeTypes: NodeType[];
  abstract isOverlayOf: ReadNodeGraph | null;
  abstract nodes: AnyNodeData[];
  abstract get size(): number;
  abstract get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  abstract getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][];

  get roots() {
    return this.nodes.filter((n) => n.parentPtr == null || this.get(n.parentPtr) == null);
  }

  describeSelf(): string {
    const rootsStr = this.roots.map(describeNode).join(", ") || "<no roots>";
    const nodeTypesStr = this.nodeTypes.map((t) => toCamelName(NodeType, t)).join("|");
    return `${this.constructor.name}(${rootsStr}, ${this.size} nodes, ${nodeTypesStr} ${describeScope(this.scope)})`;
  }

  getOrError<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] {
    const node = this.get(key);
    if (node == null) throw new Error(`node ${describeNode(key)} not found in ${this.describeSelf()}`);
    return node;
  }

  getMaybe<T extends NodeType>(key: NodeKey<T> | undefined | null): NodeTypeMapping[T] | null {
    return key ? this.get(key) : null;
  }

  has(node: NodeKey<any>): boolean {
    return this.get(node) != null;
  }

  getAncestors<T extends NodeType = NodeType>(
    node: NodeKey<any>,
    options?: { metatypes?: T[]; includeSelf?: boolean; root?: NodeKey<any> },
  ): NodeTypeMapping[T][] {
    const ancestors: NodeTypeMapping[T][] = [];
    node = options?.includeSelf ? this.get(node) : this.get(node)?.parentPtr;
    while (node != null && (options?.root == null || (node.id != options.root.id && node.ck != options.root.ck))) {
      const parentNode = this.get(node);
      if (parentNode == null) break;
      if (options?.metatypes == null || options?.metatypes.includes(parentNode.metatype as unknown as T)) {
        ancestors.push(parentNode as NodeTypeMapping[T]);
      }
      node = parentNode.parentPtr;
    }
    return ancestors;
  }

  getDescendants<T extends NodeType = NodeType>(
    node: NodeKey<any>,
    options?: { metatypes?: T[]; filter?: (node: NodeTypeMapping[T]) => boolean; includeSelf?: boolean },
  ): NodeTypeMapping[T][] {
    const descendants: NodeTypeMapping[T][] = [];
    const children = [];
    const metatypes =
      options?.metatypes ?? CHILD_NODE_TYPES[(this.get(node)?.metatype as NodeType) ?? NodeType.UNSPECIFIED];
    for (const metatype of metatypes) {
      children.push(...this.getChildren(node, metatype));
    }
    for (const child of children) {
      if (options?.filter == null || options?.filter(child as NodeTypeMapping[T])) {
        descendants.push(child as NodeTypeMapping[T]);
        descendants.push(...this.getDescendants(child, options));
      }
    }
    return descendants;
  }

  //
  // Observable helpers
  //

  abstract subscribe(
    key: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void;
  abstract subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void;

  // subscribeAncestors is generally just based on subscribe, so we can provide a default
  subscribeAncestors(
    node: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
  ): () => void {
    // NOTE: sync with other subscribeAncestors implementation (except for get/getUnfiltered)
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      let parentPtr: { id?: string } | undefined = node;
      while (parentPtr != null) {
        subs.push(this.subscribe(parentPtr, trigger, { ignoreAncestors: true }));
        parentPtr = this.get(parentPtr)?.parentPtr;
      }
    };
    const trigger = () => (update(), callback());
    update();
    return unsub;
  }

  getRef<T extends NodeType>(
    key: MaybeRef<NodeKey<T> | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<NodeKey<T> | null>;
    let sub: (() => void) | null = null;
    const unsub: () => void = () => (sub != null ? (sub(), (sub = null)) : null);
    const get = () => (keyRef.value != null ? this.get(keyRef.value as NodeKey<T>) : null);
    const update = () => {
      unsub();
      if (keyRef.value) sub = this.subscribe(keyRef.value, trigger, options);
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watchValue(keyRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  getManyRef<T extends NodeType>(
    keys: MaybeRef<NodeKey<T>[] | null | undefined>,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T][]> {
    const keysRef = toRef(keys) as Ref<NodeKey<T>[] | null | undefined>;
    const subs: (() => void)[] = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const get = () =>
      keysRef.value != null
        ? (keysRef.value.map((key) => this.get(key)).filter((n) => n != null) as NodeTypeMapping[T][])
        : [];
    const update = () => {
      unsub();
      if (keysRef.value)
        keysRef.value.filter((k) => k != null).forEach((key) => subs.push(this.subscribe(key, trigger, options)));
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watchValue(keysRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  getManyMaybeRef<T extends NodeType>(
    keys: MaybeRef<(NodeKey<T> | null | undefined)[] | null | undefined>,
    options?: NodeSubscriptionOptions | undefined,
  ): SubRef<(NodeTypeMapping[T] | null)[]> {
    const keysRef = toRef(keys) as Ref<(NodeKey<T> | null | undefined)[] | null | undefined>;
    const subs: (() => void)[] = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const get = () =>
      keysRef.value != null
        ? (keysRef.value.map((key) => (key != null ? this.get(key) : null)) as (NodeTypeMapping[T] | null)[])
        : [];
    const update = () => {
      unsub();
      if (keysRef.value)
        keysRef.value.filter((k) => k != null).forEach((key) => subs.push(this.subscribe(key!, trigger, options)));
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watchValue(keysRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  getChildrenRef<T extends NodeType>(
    parent: MaybeRef<NodeKey<any> | null>,
    metatype: T,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T][]> {
    const parentRef = toRef(parent);
    let sub: (() => void) | null = null;
    const unsub: () => void = () => (sub != null ? (sub(), (sub = null)) : null);
    const get: () => NodeTypeMapping[T][] = () =>
      parentRef.value != null ? this.getChildren(parentRef.value, metatype) : [];
    const update = () => {
      unsub();
      if (parentRef.value) sub = this.subscribeChildren(parentRef.value, metatype, trigger, options);
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watchValue(parentRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  getAncestorsRef<T extends NodeType>(
    key: MaybeRef<NodeKey<any> | null>,
    options?: { metatypes?: T[]; includeSelf?: boolean; root?: MaybeRef<NodeKey<any> | null | undefined> },
  ): SubRef<NodeTypeMapping[T][]> {
    const keyRef = toRef(key);
    const rootRef = toRef(options?.root);
    let sub: (() => void) | null = null;
    const unsub: () => void = () => (sub != null ? (sub(), (sub = null)) : null);
    const get: () => NodeTypeMapping[T][] = () =>
      keyRef.value != null
        ? this.getAncestors(keyRef.value, {
            metatypes: options?.metatypes,
            includeSelf: options?.includeSelf,
            root: rootRef.value ?? undefined,
          })
        : [];
    const update = () => {
      unsub();
      if (keyRef.value) sub = this.subscribeAncestors(keyRef.value, trigger);
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watchValue([rootRef, keyRef], () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }
}

/**
 * Core in-memory node graph without regard for hidden nodes or multi-graphs (deleted, archived, etc.).
 * If it's an overlay, we don't try to maintain local consistency (as this is likely an overlay in a layered graph).
 */
export class NodeGraph extends BaseNodeGraphMixin implements ReadNodeGraph, WriteNodeGraph {
  public readonly scope: GraphScopeData;
  public readonly nodeTypes: NodeType[];
  public readonly isOverlayOf: ReadNodeGraph | null;

  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByCk: { [ck: string]: string } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private rootsIds: string[] = [];
  private subsById: { [id: string]: Array<NodeGraphCallback> } = {};
  private subsByCk: { [ck: string]: Array<NodeGraphCallback> } = {};
  private subsByParentIdAndType: { [parentId: string]: { [type: string]: Array<NodeGraphCallback> } } = {};

  constructor(init: { scope: GraphScopeData; nodeTypes: NodeType[]; isOverlayOf?: ReadNodeGraph }) {
    super();
    this.scope = init?.scope ?? {};
    this.nodeTypes = init?.nodeTypes ?? [];
    this.isOverlayOf = init?.isOverlayOf ?? null;
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
    // we need to know existing to detect & notify move updates correctly
    const existingSelf: AnyNodeData | null = this.nodesById[node.id] ?? this.nodesByCk[(node as any).ck];
    let existingBase: AnyNodeData | null = null;
    if (!existingSelf) {
      if (this.isOverlayOf != null) {
        existingBase = this.isOverlayOf.get({ id: node.id, ck: (node as any).ck });
        if (!existingBase)
          throw new Error(
            `node ${describeNode(node)} not found in overlay ${this.describeSelf()} or base ${this.isOverlayOf.describeSelf()}`,
          );
      } else {
        throw new Error(`node ${describeNode(node)} not found in ${this.describeSelf()}`);
      }
    }

    if (existingSelf?.parentPtr?.id != node.parentPtr?.id) {
      // move
      if (existingSelf?.parentPtr != null) this._removeFromParent(existingSelf);
      if (node.parentPtr != null) this._addToParent(node);
      this.nodesById[node.id] = node;
      if ("ck" in node) this.nodesByCk[node.ck] = node.id;
      this.notify(existingSelf ?? existingBase);
      this.notify(node);
    } else {
      // update
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

    // notify all subs (they unsubscribe themselves)
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
  }

  private _addToParent(node: AnyNodeData) {
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      if (!this.nodesById[parentId] && !this.isOverlayOf && this.nodeTypes.includes(node.parentPtr.type)) {
        throw new Error(
          `parent ${describeNode(node.parentPtr)} not found in ${this.describeSelf()} for node ${describeNode(node)}`,
        );
      }
      if (!this.nodesByParentIdAndType[parentId]) this.nodesByParentIdAndType[parentId] = {};
      if (!this.nodesByParentIdAndType[parentId][node.metatype])
        this.nodesByParentIdAndType[parentId][node.metatype] = [];
      this.nodesByParentIdAndType[parentId][node.metatype].push(node.id);
    } else {
      this.rootsIds.push(node.id);
    }
  }

  private _removeFromParent(node: AnyNodeData) {
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      const nodeIdx = this.nodesByParentIdAndType[parentId]?.[node.metatype]?.findIndex((n) => n == node.id);
      if (nodeIdx == null && (this.isOverlayOf || this.nodeTypes.includes(node.parentPtr.type))) return;
      else if (nodeIdx == -1)
        throw new Error(
          `node ${describeNode(node)} not found in parent ${describeNode(node.parentPtr)} in ${this.describeSelf()}`,
        );
      this.nodesByParentIdAndType[parentId][node.metatype].splice(nodeIdx, 1);
      if (this.nodesByParentIdAndType[parentId][node.metatype].length == 0)
        delete this.nodesByParentIdAndType[parentId][node.metatype];
      if (Object.keys(this.nodesByParentIdAndType[parentId]).length == 0) delete this.nodesByParentIdAndType[parentId];
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
    const node = this.nodesById[id];
    if (node == null) return null;
    else return node as NodeTypeMapping[T];
  }

  getMany<T extends NodeType>(keys: NodeKey<T>[]): NodeTypeMapping[T][] {
    return keys.map((key) => this.get(key)).filter((n) => n != null) as NodeTypeMapping[T][];
  }

  getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][] {
    if (metatype == null) {
      const children = [];
      for (const metatype in this.nodesByParentIdAndType[parent.id!]) {
        for (const id of this.nodesByParentIdAndType[parent.id!][metatype]) {
          children.push(this.nodesById[id]);
        }
      }
      defaultSortNode(children);
      return children as NodeTypeMapping[T][];
    } else {
      const childrenIds = this.nodesByParentIdAndType[parent.id!]?.[metatype];
      if (!childrenIds) return [];
      const children = childrenIds.map((id) => this.nodesById[id]) as NodeTypeMapping[T][];
      defaultSortNode(children);
      return children;
    }
  }

  subscribe(key: { id?: string; ck?: string }, callback: NodeGraphCallback): () => void {
    // subscribe
    if (key.id) {
      if (!this.subsById[key.id]) this.subsById[key.id] = [];
      this.subsById[key.id].push(callback);
    }
    if (key.ck) {
      if (!this.subsByCk[key.ck]) this.subsByCk[key.ck] = [];
      this.subsByCk[key.ck].push(callback);
    }

    // unsubscribe
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
    callback: NodeGraphCallback,
  ): () => void {
    // subscribe
    if (!parent.id) throw new Error("parent must have an id");
    if (!this.subsByParentIdAndType[parent.id]) this.subsByParentIdAndType[parent.id] = {};
    if (!this.subsByParentIdAndType[parent.id][metatype]) this.subsByParentIdAndType[parent.id][metatype] = [];
    this.subsByParentIdAndType[parent.id][metatype].push(callback);

    // unsubscribe
    return () => {
      if (!parent.id) throw new Error("parent must have an id");
      const idx = this.subsByParentIdAndType[parent.id]?.[metatype]?.indexOf(callback);
      if (idx == null || idx < 0) throw new Error("callback not found");
      this.subsByParentIdAndType[parent.id][metatype].splice(idx, 1);
      // cleanup
      if (this.subsByParentIdAndType[parent.id][metatype].length == 0)
        delete this.subsByParentIdAndType[parent.id][metatype];
      if (Object.keys(this.subsByParentIdAndType[parent.id]).length == 0) delete this.subsByParentIdAndType[parent.id];
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

  countSubscribers() {
    return this.countDirectSubscribers() + this.countChildrenSubscribers();
  }

  countDirectSubscribers() {
    return Object.keys(this.subsById).length + Object.keys(this.subsByCk).length;
  }

  countChildrenSubscribers() {
    let count = 0;
    for (const parentId in this.subsByParentIdAndType) {
      for (const metatype in this.subsByParentIdAndType[parentId]) {
        count += this.subsByParentIdAndType[parentId][metatype].length;
      }
    }
    return count;
  }
}

/**
 * A filtered node graph.
 * For consistency this should be the outermost graph, since outer layers cannot un-hide nodes they can't see.
 * (i.e. layers further out can only be more restrictive, not less).
 * TODO :Performance: filtering node graphs is somewhat inefficient because we throw away and recompute a lot of info
 *  For instance, currently we only need deletedAt & archivedAt but we assemble all properties.
 *  Also we subscribe to any change in the ancestors, not just a visibility change?
 *  see :NodeFiltering
 * */
abstract class FilterBaseNodeGraphMixin extends BaseNodeGraphMixin {
  public readonly filter: ShallowRef<NodeGraphFilter>;

  constructor(filter: MaybeRef<NodeGraphFilter>) {
    super();
    this.filter = isRef(filter) ? filter : shallowRef(filter);
  }

  get roots() {
    // unlike in general graphs, here we need to walk the graph properly to respect the filters
    const roots: AnyNodeData[] = [];
    for (const node of this.nodes) {
      // node is root if itself is visible but none of its ancestors are
      if (node.parentPtr == null) {
        roots.push(node);
      } else {
        const parentNode = this.getUnfiltered(node.parentPtr);
        if (parentNode == null || !this.isNodeVisibleSelf(parentNode)) {
          roots.push(node);
        }
      }
    }
    return roots;
  }

  /** Get the node at the given key without applying any filters */
  abstract getUnfiltered<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null;

  /** Get the node at the given key (considering filters) */
  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const node = this.getUnfiltered(key);
    if (node == null || !this.isNodeVisibleAbsolute(node)) return null;
    else return node;
  }

  /** Checks whether the node itself is visible according to its own state */
  protected isNodeVisibleSelf(node: AnyNodeData): boolean {
    return this.filter.value.includeHidden || (node.deletedAt == null && node.archivedAt == null);
  }

  /** Checks whether the node itself *and* all of its ancestors are visible */
  protected isNodeVisibleAbsolute(node: AnyNodeData): boolean {
    if (!this.isNodeVisibleSelf(node)) return false;
    let parent = node.parentPtr;
    while (parent != null) {
      const parentNode = this.getUnfiltered(parent);
      if (parentNode == null) break; // parent not in this graph
      if (!this.isNodeVisibleSelf(parentNode)) return false;
      parent = parentNode.parentPtr;
    }
    return true;
  }

  subscribeAncestors(
    node: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
  ): () => void {
    // NOTE: sync with other subscribeAncestors implementation (except for get/getUnfiltered)
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      let parentPtr: { id?: string } | undefined = node;
      while (parentPtr != null) {
        subs.push(this.subscribe(parentPtr, trigger, { ignoreAncestors: true }));
        parentPtr = this.getUnfiltered(parentPtr)?.parentPtr;
      }
    };
    const trigger = () => {
      update();
      callback();
    };
    update();
    return unsub;
  }
}

/**
 * A proxy to a single graph (like a LayerNodeGraph with a single layer).
 */
export class ProxyNodeGraph extends FilterBaseNodeGraphMixin implements ReadNodeGraph {
  readonly _graph: ShallowRef<ReadNodeGraph | null>;

  constructor(init: { graph?: MaybeRef<ReadNodeGraph | null>; filter: MaybeRef<NodeGraphFilter> }) {
    super(init.filter);
    this._graph = isRef(init.graph) ? init.graph : shallowRef(init.graph ?? null);
  }

  public get graph(): ReadNodeGraph | null {
    return this._graph.value;
  }

  public set graph(graph: ReadNodeGraph | null) {
    if (this._graph.value !== graph) this._graph.value = graph;
  }

  get scope(): GraphScopeData {
    return this._graph.value?.scope ?? EMPTY_SCOPE;
  }

  get nodeTypes(): NodeType[] {
    return this._graph.value?.nodeTypes ?? [];
  }

  get isOverlayOf(): ReadNodeGraph | null {
    return this._graph.value?.isOverlayOf ?? null;
  }

  get nodes(): AnyNodeData[] {
    return this._graph.value?.nodes ?? [];
  }

  get size(): number {
    return this._graph.value?.size ?? 0;
  }

  getUnfiltered<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    return this._graph.value?.get(key) ?? null;
  }

  getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][] {
    if (!this.get(parent)) return [];
    // getChildren but with the filter
    const children = this._graph.value?.getChildren(parent, metatype)?.filter((n) => this.isNodeVisibleSelf(n)) ?? [];
    return children;
  }

  getDescendants<T extends NodeType = NodeType>(
    node: NodeKey<any>,
    options?: { metatypes?: T[]; filter?: (node: NodeTypeMapping[T]) => boolean; includeSelf?: boolean },
  ): NodeTypeMapping[T][] {
    // augment with the filter
    return super.getDescendants(node, {
      metatypes: options?.metatypes,
      filter:
        options?.filter != null
          ? (n) => this.isNodeVisibleSelf(n) && options.filter!(n)
          : (n) => this.isNodeVisibleSelf(n),
      includeSelf: options?.includeSelf,
    });
  }

  subscribe(
    key: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      if (this._graph.value != null) {
        subs.push(this._graph.value.subscribe(key, callback));
        if (!options?.ignoreAncestors) subs.push(this.subscribeAncestors(key, callback));
      }
    };
    update();

    const graphSub = watch(this._graph, () => (callback(), update()), { flush: "sync" });
    const filterSub = watch(this.filter, callback, { flush: "sync" });
    return () => (unsub(), graphSub(), filterSub());
  }

  subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      if (this._graph.value != null) {
        subs.push(this._graph.value.subscribeChildren(parent, metatype, callback));
        if (!options?.ignoreAncestors) subs.push(this.subscribeAncestors(parent, callback));
      }
    };
    update();

    const graphSub = watch(this._graph, () => (callback(), update()), { flush: "sync" });
    const filterSub = watch(this.filter, callback, { flush: "sync" });
    return () => (unsub(), graphSub(), filterSub());
  }
}

/**
 * A graph composed of multiple (potentially overlapping subgraphs).
 * Nodes are merged from the layers in order, with later layers taking precedence.
 */
export class LayerNodeGraph extends FilterBaseNodeGraphMixin implements ReadNodeGraph {
  // TODO :Performance: LayerNodeGraph.layers could be scoped?
  //  (so we only need to acquire refs from layers with the requested scope)
  public readonly layers: ShallowRef<ReadNodeGraph[]>;

  constructor(init: { layers?: MaybeRef<ReadNodeGraph[]>; filter: MaybeRef<NodeGraphFilter> }) {
    super(init.filter);
    this.layers = !isRef(init.layers) ? shallowRef(init.layers ?? []) : init.layers;
  }

  get isOverlayOf(): ReadNodeGraph | null {
    return null;
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

  get scope(): GraphScopeData {
    if (this.layers.value.length == 0) return {} as GraphScopeData;
    else return this.layers.value[0].scope;
  }

  get nodeTypes(): NodeType[] {
    if (this.layers.value.length == 0) return [];
    else return this.layers.value[0].nodeTypes;
  }

  getUnfiltered<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    let mergedNode: NodeTypeMapping[T] | null = null;
    for (const layer of this.layers.value) {
      // merge node from next layer
      const node = layer.get(key);
      if (node != null) {
        if (!mergedNode) mergedNode = node;
        else mergedNode = mergeNode(mergedNode, node);
      }
    }
    return mergedNode;
  }

  getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][] {
    if (!this.get(parent)) return [];
    const mergedChildrenById: { [id: string]: NodeTypeMapping[T] } = {};
    for (const layer of this.layers.value) {
      // check whether children are still at the same parent in this layer
      // NOTE :Performance: cross-checking layers for every child seems inefficient :NodeFiltering
      for (const childId of Object.keys(mergedChildrenById)) {
        const child = layer.get({ id: childId });
        if (child != null && child.parentPtr?.id != parent.id) {
          delete mergedChildrenById[childId];
        }
      }

      // merge children from next layer
      const children = layer.getChildren(parent, metatype);
      for (const child of children) {
        if (!mergedChildrenById[child.id]) {
          mergedChildrenById[child.id] = child;
        } else {
          mergedChildrenById[child.id] = mergeNode(mergedChildrenById[child.id], child);
        }
      }
    }
    const children = Object.values(mergedChildrenById).filter((n) => this.isNodeVisibleSelf(n));
    defaultSortNode(children);
    return children;
  }

  subscribe(
    key: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      this.layers.value.forEach((layer) => {
        subs.push(layer.subscribe(key, callback));
      });
      if (!options?.ignoreAncestors) subs.push(this.subscribeAncestors(key, callback));
    };
    update();

    const graphSub = watch(this.layers, () => (callback(), update()), { flush: "sync" });
    const filterSub = watch(this.filter, callback, { flush: "sync" });
    return () => (unsub(), graphSub(), filterSub());
  }

  subscribeChildren<T extends NodeType>(
    parent: { id?: string | undefined; ck?: string | undefined },
    metatype: T,
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      this.layers.value.forEach((layer) => {
        subs.push(layer.subscribeChildren(parent, metatype, callback));
      });
      if (!options?.ignoreAncestors) subs.push(this.subscribeAncestors(parent, callback));
    };
    update();

    const graphSub = watch(this.layers, () => (callback(), update()), { flush: "sync" });
    const filterSub = watch(this.filter, callback, { flush: "sync" });
    return () => (unsub(), graphSub(), filterSub());
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

/** Resolve the node in the given graph if it's a reference */
export function resolveNode(graph: ReadNodeGraph, node: AnyNodeData | AnyNodeReferenceData): AnyNodeData {
  return isNodeRef(node) ? graph.getOrError(node) : node;
}

export type NodeTreeItem<T extends NodeType> = {
  id: string;
  node: NodeTypeMapping[T];
  nodePtr: TypedNodeReferenceData<T>;
  depth: number;
  hasChildren: boolean;
};

/** Get the selectively expanded descendants of a root (reactively)  */
export function walkDescendantsRef<T extends NodeType>(walk: {
  graph: ReadNodeGraph;
  rootPtr: Ref<NodeKey<any> | null | undefined>;
  nodeTypes: Ref<T[]>;
  isExpanded: (node: NodeTypeMapping[T]) => boolean;
  isIncludedSelf: (node: NodeTypeMapping[T]) => boolean;
  isIncludedChildren: (node: NodeTypeMapping[T]) => boolean;
  watchSource?: WatchSource<any>;
}): { items: Ref<NodeTreeItem<T>[]>; trigger: () => void } {
  type NodeT = NodeTypeMapping[T];
  type ItemT = NodeTreeItem<T>;

  const { graph, rootPtr, nodeTypes, isExpanded, isIncludedSelf, isIncludedChildren } = walk;

  const subs: Array<() => void> = [];
  const unsub = () => {
    subs.forEach((sub) => sub());
    subs.length = 0;
  };
  /** Walk everything from scratch (as needed) */
  function get(): ItemT[] {
    unsub();
    if (rootPtr.value == null) return [];

    const items: ItemT[] = [];
    function walkDescendants(node: NodeT, depth: number) {
      // make item
      const children = nodeTypes.value.flatMap((nodeType) => graph.getChildren(node, nodeType)).filter(isIncludedSelf);
      const item: ItemT = {
        id: node.id,
        node,
        nodePtr: toNodeRef(node)!,
        depth,
        hasChildren: children.length > 0,
      };
      if (depth >= 0) items.push(item); // ignore root

      // descend
      // NOTE: we need to subscribe one extra 'down' for 'hasChildren' above
      nodeTypes.value.forEach((nodeType) =>
        subs.push(graph.subscribeChildren(node, nodeType, trigger, { ignoreAncestors: true })),
      );
      if (depth < 0 || isExpanded(node)) {
        children.forEach((child) => {
          if (isIncludedChildren(child)) walkDescendants(child, depth + 1);
          else
            items.push({
              id: child.id,
              node: child,
              nodePtr: toNodeRef(child),
              depth: depth + 1,
              hasChildren: false,
            });
        });
      }
    }

    // collect
    const root = graph.getMaybe(rootPtr.value);
    subs.push(graph.subscribe(rootPtr.value, trigger));
    if (root != null) walkDescendants(root, -1); // ignore root

    return items;
  }

  const { ref: items, trigger } = manualSubRef(get, unsub);
  watch(rootPtr, trigger);
  if (walk.watchSource) watch(walk.watchSource, trigger);
  tryOnBeforeUnmount(unsub);

  return { items: items, trigger };
}

/** Get the direct children of a dynamic set of nodes */
export function getGroupedChildrenRef<T extends NodeType>(walk: {
  graph: ReadNodeGraph;
  parentPtrs: Ref<NodeKey<any>[]> | Ref<NodeTreeItem<any>[]>;
  childTypes: T[];
}): { childrenByParentId: Ref<{ [parentId: string]: NodeTypeMapping[T][] }>; trigger: () => void } {
  type NodeT = NodeTypeMapping[T];

  const subs: Array<() => void> = [];
  const unsub = () => {
    subs.forEach((sub) => sub());
    subs.length = 0;
  };

  const parentIds = computedValue(() => walk.parentPtrs.value.map((p) => p.id!));

  function get(): { [parentId: string]: NodeT[] } {
    unsub();

    const childrenByParentId: { [parentId: string]: NodeT[] } = {};
    for (const parentId of parentIds.value) {
      for (const childType of walk.childTypes) {
        const children = walk.graph.getChildren({ id: parentId }, childType);
        if (children.length > 0) {
          if (!childrenByParentId[parentId]) childrenByParentId[parentId] = [];
          childrenByParentId[parentId].push(...children);
        }
        subs.push(walk.graph.subscribeChildren({ id: parentId }, childType, trigger));
      }
    }

    return childrenByParentId;
  }

  const { ref: childrenByParentId, trigger } = manualSubRef(get, unsub);
  watch(parentIds, trigger);

  return { childrenByParentId, trigger };
}

/** Whether child is a descendant of parent */
export function isDescendantOf(graph: ReadNodeGraph, child: NodeKey<any>, parent: NodeKey<any>): boolean {
  return graph.getAncestors(child, { includeSelf: true }).some((ancestor) => ancestor.id == parent.id);
}

/**
 * A supergraph composed of multiple subgraphs.
 * Used for global resolution of nodes across multiple graphs.
 * Graphs are searched in order, first match wins. We assume that that there are no meaningful overlaps.
 */
export class NodeSuperGraph {
  graphs: Ref<ReadNodeGraph[]>;

  constructor(graphs: ReadNodeGraph[] = []) {
    this.graphs = shallowRef(graphs);
  }

  /**
   * Adds a graph to the supergraph.
   */
  addGraph(graph: ReadNodeGraph) {
    this.graphs.value.push(graph);
    triggerRef(this.graphs);
  }

  /**
   * Removes a graph from the supergraph.
   */
  removeGraph(graph: ReadNodeGraph) {
    const idx = this.graphs.value.indexOf(graph);
    if (idx >= 0) {
      this.graphs.value.splice(idx, 1);
      triggerRef(this.graphs);
    }
  }

  /**
   * Gets a node from the supergraph.
   */
  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    for (const graph of this.graphs.value) {
      const node = graph.get(key);
      if (node != null) return node;
    }
    return null;
  }

  /**
   * Gets a node from the supergraph maybe.
   */
  getMaybe<T extends NodeType>(key: NodeKey<T> | null): NodeTypeMapping[T] | null {
    if (key == null) return null;
    else return this.get(key);
  }

  /**
   * Subscribe to any change in the given key
   * Unlike ReadNodeGraph, we only subscribe to the first graph containing the node if it exists (otherwise all).
   */
  subscribe(key: { id?: string; ck?: string }, callback: NodeGraphCallback): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.splice(0, subs.length);
    };
    const update = () => {
      unsub();
      let found = false;
      for (const graph of this.graphs.value) {
        const node = graph.get(key);
        if (node != null) {
          subs.push(graph.subscribe(key, callback));
          found = true;
          break;
        }
      }
      if (!found) {
        // subscribe to all graphs and graphs list
        for (const graph of this.graphs.value) {
          subs.push(graph.subscribe(key, () => (callback(), update())));
        }
        subs.push(watch(this.graphs, () => (callback(), update()), { flush: "sync" }));
      }
    };
    update();
    return unsub;
  }

  /**
   * Gets a reactive reference to the current node with that key
   */
  getRef<T extends NodeType>(key: MaybeRef<NodeKey<T> | null>): Ref<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<NodeKey<T>>;
    let sub: (() => void) | null = null;
    const unsub = () => (sub != null ? (sub(), (sub = null)) : null);
    const get = () => (keyRef.value != null ? this.get(keyRef.value) : null);
    const update = () => {
      unsub();
      if (keyRef.value != null) sub = this.subscribe(keyRef.value, trigger);
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watch(keyRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }
}
