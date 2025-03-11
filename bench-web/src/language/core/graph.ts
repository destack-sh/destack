import { toCamelName } from "@/language/core/const";
import { defaultSortNode } from "@/language/core/order";
import {
  CHILD_NODE_TYPES,
  GraphScopeData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  type AnyNodeData,
  type NodeTypeMapping,
} from "@/proto/wire";
import {
  describeNode,
  describeScope,
  EMPTY_SCOPE,
  isNodeRef,
  propertyInfo,
  toNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { ConnectionBase } from "@/system/connection";
import { computedValue, manualSubRef, watchValue, type SubRef } from "@/utils/ref";
import { tryOnBeforeUnmount } from "@vueuse/core";
import {
  computed,
  isRef,
  shallowRef,
  toRef,
  watch,
  type MaybeRef,
  type Ref,
  type ShallowRef,
  type WatchSource,
} from "vue";

/** A NodeReference but with proper typing */
export type NodeKey<T extends NodeType> = Pick<NodeReferenceData, "id" | "ck" | "baseId"> & { nodeType?: T };
export type TypedNodeKey<T extends NodeType> = NodeKey<T> & { nodeType: T };

// NOTE :Performance!: should differentiate node update types for :NodeFiltering
//  (e.g. full, create/delete, move, update:[properties...], etc.)
export type NodeGraphCallback = () => void;
export type NodeSubscriptionOptions = {
  ignoreAncestors?: boolean;
  id?: any;
};

/** A filter for nodes in a graph. Nodes pretend to not be in the graph when this predicate fails. */
export type NodeGraphFilter = {
  /** Hidden = deletedAt */
  includeDeleted: boolean;
};
export const DEFAULT_NODE_FILTER = { includeDeleted: false };
export const PASSTHROUGH_NODE_FILTER = { includeDeleted: true };

/** A node graph with read methods */
export interface ReadNodeGraph {
  describeSelf(): string;

  /** The scope contained in this graph */
  get scope(): GraphScopeData;

  /** The node types contained in this graph */
  get nodeTypes(): Set<NodeType>;

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

  /** Gets many nodes with the given keys (not reactive) */
  getMany<T extends NodeType>(nodes: NodeKey<T>[]): NodeTypeMapping[T][];

  /** Gets many nodes with the given keys, filtering out missing nodes (not reactive) */
  getManyMaybe<T extends NodeType>(nodes: NodeKey<T>[]): NodeTypeMapping[T][];

  /** Gets the children of the given parent with the given metatype (not reactive) */
  getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][];

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

  /** Gets nodes of a specific type */
  getOfType<T extends NodeType>(metatype: T): NodeTypeMapping[T][];

  //
  // Observable helpers
  //

  /** Subscribe to any changein the graph */
  subscribeAny(callback: NodeGraphCallback, options?: NodeSubscriptionOptions): () => void;

  /** Subscribe to any change in the given node */
  subscribe(
    node: { id?: string; ck?: string },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void;

  /** Subscribe to any change in the given node's children */
  subscribeChildren<T extends NodeType>(
    node: { id?: string; ck?: string },
    metatype: T,
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void;

  /** Subscribes to any change in the given node's ancestors */
  subscribeAncestors(
    node: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
  ): () => void;

  /** Gets a reactive reference to the current node with that key */
  getRef<T extends NodeType>(
    node: MaybeRef<NodeKey<T> | undefined | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T] | null>;

  /** Gets a reactive reference to many nodes with the given keys (missing nodes excluded) */
  getManyRef<T extends NodeType>(
    nodes: MaybeRef<NodeKey<T>[] | undefined | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T][]>;

  /** Gets a reactive reference to many nodes with potentially unset keys (missing nodes included as null) */
  getManyMaybeRef<T extends NodeType>(
    nodes: MaybeRef<(NodeKey<T> | undefined | null)[] | undefined | null>,
    options?: NodeSubscriptionOptions,
  ): SubRef<(NodeTypeMapping[T] | null)[]>;

  /** Gets a reactive reference to the children of the given parent with the given metatype */
  getChildrenRef<T extends NodeType>(
    node: MaybeRef<NodeKey<any> | undefined | null>,
    metatype: T,
    options?: NodeSubscriptionOptions,
  ): SubRef<NodeTypeMapping[T][]>;

  /** Gets ancestors reactively */
  getAncestorsRef<T extends NodeType>(
    node: MaybeRef<NodeKey<any> | undefined | null>,
    options?: { metatypes?: T[]; includeSelf?: boolean; root?: MaybeRef<NodeKey<any> | undefined | null> },
  ): SubRef<NodeTypeMapping[T][]>;

  /** Gets descendants reactively */
  getDescendantsRef<T extends NodeType>(
    node: MaybeRef<NodeKey<any> | undefined | null>,
    options?: { metatypes?: T[]; includeSelf?: boolean },
  ): SubRef<NodeTypeMapping[T][]>;

  /** Gets nodes of a specific type reactively */
  getOfTypeRef<T extends NodeType>(metatype: T): SubRef<NodeTypeMapping[T][]>;
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
  abstract nodeTypes: Set<NodeType>;
  abstract isOverlayOf: ReadNodeGraph | null;
  abstract nodes: AnyNodeData[];
  abstract get size(): number;
  abstract get<T extends NodeType>(node: NodeKey<T>): NodeTypeMapping[T] | null;
  abstract getChildren<T extends NodeType = NodeType>(parent: NodeKey<any>, metatype?: T): NodeTypeMapping[T][];
  private static _id: number = 0;
  public readonly id: number;

  constructor() {
    this.id = BaseNodeGraphMixin._id++;
  }

  get roots() {
    return this.nodes.filter((n) => n.parentPtr == null || this.get(n.parentPtr) == null);
  }

  describeSelf(): string {
    const rootsStr = this.roots.map(describeNode).join(", ") || "<no roots>";
    const nodeTypesStr = Array.from(this.nodeTypes)
      .map((t) => toCamelName(NodeType, t))
      .join("|");
    return `${this.constructor.name}(${rootsStr}, ${this.size} nodes, ${nodeTypesStr} ${describeScope(this.scope)})`;
  }

  getOrError<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] {
    const node = this.get(key);
    if (node == null) throw new Error(`node ${describeNode(key)} not found in ${this.describeSelf()}`);
    return node;
  }

  getMany<T extends NodeType>(nodes: NodeKey<T>[]): NodeTypeMapping[T][] {
    return nodes.map((key) => this.getOrError(key));
  }

  getManyMaybe<T extends NodeType>(nodes: NodeKey<T>[]): NodeTypeMapping[T][] {
    return nodes.map((key) => this.get(key)).filter((n) => n != null);
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
    if (options?.includeSelf) {
      const self = this.get(node);
      if (self != null && (options?.filter == null || options?.filter(self as NodeTypeMapping[T]))) {
        descendants.push(self as NodeTypeMapping[T]);
      }
    }
    const children = [];
    const metatypes =
      options?.metatypes ?? CHILD_NODE_TYPES[(this.get(node)?.metatype as NodeType) ?? NodeType.UNSPECIFIED];
    for (const metatype of metatypes) {
      children.push(...this.getChildren(node, metatype));
    }
    for (const child of children) {
      if (options?.filter == null || options?.filter(child as NodeTypeMapping[T])) {
        descendants.push(child as NodeTypeMapping[T]);
        descendants.push(...this.getDescendants(child, { ...options, includeSelf: false }));
      }
    }
    return descendants;
  }

  getOfType<T extends NodeType>(metatype: T): NodeTypeMapping[T][] {
    return this.nodes.filter((n) => (n.metatype as unknown as NodeType) == metatype) as NodeTypeMapping[T][];
  }

  //
  // Observable helpers
  //

  abstract subscribeAny(callback: NodeGraphCallback, options?: NodeSubscriptionOptions): () => void;
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
      subs.length = 0;
    };
    const update = () => {
      unsub();
      let parentPtr: { id?: string } | undefined = node;
      while (parentPtr != null) {
        subs.push(this.subscribe(parentPtr, trigger, { ignoreAncestors: true, id: node.id }));
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
    const unsub: () => void = () => {
      sub?.();
      sub = null;
    };
    const get = () => (keyRef.value != null ? this.get(keyRef.value as NodeKey<T>) : null);
    const update = () => {
      unsub();
      if (keyRef.value) sub = this.subscribe(keyRef.value, trigger, options);
    };

    const { ref, trigger } = manualSubRef(get, unsub, options != null ? { id: options?.id } : undefined);
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

    const { ref, trigger } = manualSubRef(get, unsub, options != null ? { id: options?.id } : undefined);
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

    const { ref, trigger } = manualSubRef(get, unsub, options != null ? { id: options?.id } : undefined);
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

    const { ref, trigger } = manualSubRef(get, unsub, options != null ? { id: options?.id } : undefined);
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

  getDescendantsRef<T extends NodeType>(
    node: MaybeRef<NodeKey<any> | null>,
    options?: { metatypes?: T[]; includeSelf?: boolean },
  ): SubRef<NodeTypeMapping[T][]> {
    // NOTE :Performance: getDescendantsRef just uses subscribeAny for simplicity, but that's not ideal
    const nodeRef = toRef(node);
    const get: () => NodeTypeMapping[T][] = () =>
      nodeRef.value != null ? this.getDescendants(nodeRef.value, options) : [];
    let sub: (() => void) | null = null;
    const unsub = () => {
      if (sub != null) {
        sub();
        sub = null;
      }
    };
    const { ref, trigger } = manualSubRef(get, unsub);
    sub = this.subscribeAny(trigger);
    trigger();
    return ref;
  }

  getOfTypeRef<T extends NodeType>(metatype: T): SubRef<NodeTypeMapping[T][]> {
    // NOTE :Performance: getOfTypeRef just uses subscribeAny for simplicity, but that's not ideal
    const get: () => NodeTypeMapping[T][] = () => this.getOfType(metatype);
    let sub: (() => void) | null = null;
    const unsub = () => {
      if (sub != null) {
        sub();
        sub = null;
      }
    };
    const { ref, trigger } = manualSubRef(get, unsub);
    sub = this.subscribeAny(trigger);
    trigger();
    return ref;
  }
}

/**
 * Core in-memory node graph without regard for hidden nodes or multi-graphs (deleted, etc.).
 * If it's an overlay, we don't try to maintain local consistency (as this is likely an overlay in a layered graph).
 */
export class NodeGraph extends BaseNodeGraphMixin implements ReadNodeGraph, WriteNodeGraph {
  public readonly scope: GraphScopeData;
  public readonly nodeTypes: Set<NodeType>;
  public readonly isOverlayOf: ReadNodeGraph | null;

  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private rootsIds: string[] = [];
  private anySubs: Array<() => void> = [];
  private nodeSubsById: { [id: string]: Array<NodeGraphCallback> } = {};
  private nodeSubsByParentIdAndType: { [parentId: string]: { [type: string]: Array<NodeGraphCallback> } } = {};

  constructor(init: { scope: GraphScopeData; nodeTypes: Set<NodeType>; isOverlayOf?: ReadNodeGraph }) {
    super();
    this.scope = init?.scope ?? {};
    this.nodeTypes = init?.nodeTypes ?? new Set();
    this.isOverlayOf = init?.isOverlayOf ?? null;
  }

  add(node: AnyNodeData) {
    if (!node.id) throw new Error("node must have an id");
    if (this.nodesById[node.id])
      throw new Error(`node ${describeNode(node)} id already exists in ${this.describeSelf()}`);
    this.nodesById[node.id] = node;

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
    const existingSelf: AnyNodeData | null = this.nodesById[node.id];
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
      this.notify(existingSelf ?? existingBase);
      this.notify(node);
    } else {
      // update
      this.nodesById[node.id] = node;
      this.notify(node);
    }
  }

  remove(node: AnyNodeData) {
    if (!node.id) throw new Error(`node must have an id: ${describeNode(node)}`);
    delete this.nodesById[node.id];

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
    const nodesById = this.nodesById;
    const nodesByParentIdAndType = this.nodesByParentIdAndType;
    this.nodesById = {};
    this.nodesByParentIdAndType = {};

    // notify relevant subs
    this.anySubs.forEach((sub) => sub());
    for (const id in nodesById) {
      this.nodeSubsById[id]?.forEach((sub) => sub());
    }
    for (const parentId in nodesByParentIdAndType) {
      if (!this.nodeSubsByParentIdAndType[parentId]) continue;
      for (const metatype in this.nodeSubsByParentIdAndType[parentId]) {
        this.nodeSubsByParentIdAndType[parentId][metatype].forEach((sub) => sub());
      }
    }
  }

  private _addToParent(node: AnyNodeData) {
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      if (!this.nodesById[parentId] && !this.isOverlayOf && this.nodeTypes.has(node.parentPtr.nodeType)) {
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
      if (nodeIdx == null && (this.isOverlayOf || this.nodeTypes.has(node.parentPtr.nodeType))) return;
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
    if (!key.id) return null;
    const node = this.nodesById[key.id];
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

  subscribeAny(callback: NodeGraphCallback, options?: NodeSubscriptionOptions): () => void {
    const cb = () => callback(); // wrap to get a unique reference
    this.anySubs.push(cb);
    return () => {
      const idx = this.anySubs.indexOf(cb);
      if (idx >= 0) this.anySubs.splice(idx, 1);
    };
  }

  subscribe(key: { id?: string }, callback: NodeGraphCallback): () => void {
    const cb = () => callback(); // wrap to get a unique reference
    // subscribe
    if (key.id) {
      if (!this.nodeSubsById[key.id]) this.nodeSubsById[key.id] = [];
      this.nodeSubsById[key.id].push(cb);
    }
    // unsubscribe
    return () => {
      if (key.id) {
        if (this.nodeSubsById[key.id]) {
          this.nodeSubsById[key.id].splice(this.nodeSubsById[key.id].indexOf(cb), 1);
          if (this.nodeSubsById[key.id].length == 0) {
            delete this.nodeSubsById[key.id];
          }
        }
      }
    };
  }

  subscribeChildren<T extends NodeType>(parent: { id?: string }, metatype: T, callback: NodeGraphCallback): () => void {
    // subscribe
    if (!parent.id) throw new Error("parent must have an id");
    if (!this.nodeSubsByParentIdAndType[parent.id]) this.nodeSubsByParentIdAndType[parent.id] = {};
    if (!this.nodeSubsByParentIdAndType[parent.id][metatype]) this.nodeSubsByParentIdAndType[parent.id][metatype] = [];
    this.nodeSubsByParentIdAndType[parent.id][metatype].push(callback);

    // unsubscribe
    return () => {
      if (!parent.id) throw new Error("parent must have an id");
      const idx = this.nodeSubsByParentIdAndType[parent.id]?.[metatype]?.indexOf(callback);
      if (idx == null || idx < 0) throw new Error("callback not found");
      this.nodeSubsByParentIdAndType[parent.id][metatype].splice(idx, 1);
      // cleanup
      if (this.nodeSubsByParentIdAndType[parent.id][metatype].length == 0)
        delete this.nodeSubsByParentIdAndType[parent.id][metatype];
      if (Object.keys(this.nodeSubsByParentIdAndType[parent.id]).length == 0)
        delete this.nodeSubsByParentIdAndType[parent.id];
    };
  }

  notify(node: AnyNodeData) {
    // NOTE :Performance: NodeGraph.notify is called a lot and copies the subscription list every time
    //  (to avoid concurrent modification issues, which were very painful to debug..
    //   there must be some better way, but the entire graph & node ref system is a bit wonky and inefficient anyway)
    this.anySubs.forEach((sub) => sub());
    if (this.nodeSubsById[node.id]) {
      this.nodeSubsById[node.id].slice().forEach((sub) => sub());
    }
    if (node.parentPtr?.id && this.nodeSubsByParentIdAndType[node.parentPtr.id]) {
      const subs = this.nodeSubsByParentIdAndType[node.parentPtr.id][node.metatype];
      if (subs) {
        subs.slice().forEach((sub) => sub());
      }
    }
  }

  countSubscribers() {
    return this.countAnySubscribers() + this.countDirectSubscribers() + this.countChildrenSubscribers();
  }

  countAnySubscribers() {
    return this.anySubs.length;
  }

  countDirectSubscribers() {
    return Object.keys(this.nodeSubsById).length;
  }

  countChildrenSubscribers() {
    let count = 0;
    for (const parentId in this.nodeSubsByParentIdAndType) {
      for (const metatype in this.nodeSubsByParentIdAndType[parentId]) {
        count += this.nodeSubsByParentIdAndType[parentId][metatype].length;
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
    return this.filter.value.includeDeleted || node.deletedAt == null;
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
      subs.length = 0;
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

  get nodeTypes(): Set<NodeType> {
    return this._graph.value?.nodeTypes ?? new Set();
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

  subscribeAny(callback: NodeGraphCallback, options?: NodeSubscriptionOptions): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const update = () => {
      unsub();
      if (this._graph.value != null) {
        subs.push(this._graph.value.subscribeAny(callback, options));
      }
    };
    update();

    const graphSub = watch(this._graph, () => (callback(), update()), { flush: "sync" });
    const filterSub = watch(this.filter, callback, { flush: "sync" });
    return () => (unsub(), graphSub(), filterSub());
  }

  subscribe(
    key: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const update = () => {
      unsub();
      if (this._graph.value != null) {
        if (!options?.ignoreAncestors) {
          subs.push(this.subscribeAncestors(key, callback));
        } else {
          subs.push(this._graph.value.subscribe(key, callback, { ignoreAncestors: true }));
        }
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
      subs.length = 0;
    };
    const update = () => {
      unsub();
      if (this._graph.value != null) {
        subs.push(this._graph.value.subscribeChildren(parent, metatype, callback));
        if (!options?.ignoreAncestors) {
          subs.push(this.subscribeAncestors(parent, callback));
        }
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

  get nodeTypes(): Set<NodeType> {
    if (this.layers.value.length == 0) return new Set();
    else return this.layers.value[0].nodeTypes;
  }

  getUnfiltered<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    let mergedNode: NodeTypeMapping[T] | null = null;
    for (const layer of this.layers.value) {
      // merge node from next layer
      const node = layer instanceof FilterBaseNodeGraphMixin ? layer.getUnfiltered(key) : layer.get(key);
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

  subscribeAny(callback: NodeGraphCallback, options?: NodeSubscriptionOptions): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const update = () => {
      unsub();
      this.layers.value.forEach((layer) => {
        subs.push(layer.subscribeAny(callback, options));
      });
    };
    update();

    const graphSub = watch(this.layers, () => (callback(), update()), { flush: "sync" });
    const filterSub = watch(this.filter, callback, { flush: "sync" });
    return () => (unsub(), graphSub(), filterSub());
  }

  subscribe(
    key: { id?: string | undefined; ck?: string | undefined },
    callback: NodeGraphCallback,
    options?: NodeSubscriptionOptions,
  ): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const update = () => {
      unsub();

      // NOTE :Architecture :Robustness: node subscriptions are/were not always reliably updated if ignoreAncestors (?) :NodeRefStability
      this.layers.value.forEach((layer) => {
        subs.push(layer.subscribe(key, callback));
        if (!options?.ignoreAncestors) subs.push(layer.subscribeAncestors(key, callback));
      });
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
      subs.length = 0;
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

export type PartialNode<T extends NodeType> = NodeTypeMapping[T] & { setPaths?: string[][] };

/**
 * Creates a new node with the explicitly set paths from the overlay superimposed on the base.
 */
export function mergeNode<T extends NodeType>(base: PartialNode<T>, partial: PartialNode<T>): NodeTypeMapping[T] {
  if (partial.setPaths != null && partial.setPaths.length > 0) {
    const merged: NodeTypeMapping[T] = structuredClone(base);
    for (const path of partial.setPaths) {
      const rootProperty = propertyInfo(base.metatype, Number(path[0]));
      let partialObj: any = partial;
      let mergedObj: any = merged;
      let key = path[0];
      for (let i = 0; i < path.length; i++) {
        // map key
        key = path[i];
        const propId = Number(key);
        const isProperty = !Number.isNaN(propId) && (i == 0 || !rootProperty.isValuePacked);
        if (isProperty) {
          // builtin object property
          const objProperties = PROPERTY_ENUM_BY_TYPE[mergedObj.metatype as ObjectType];
          const propName = objProperties?.[propId];
          if (propName == null) {
            break; // invalid path
          }
          key = propName;
        }
        if (i < path.length - 1) {
          // descend into value
          let nextMergedObj = mergedObj[key];
          let nextPartialObj = partialObj[key];
          if (nextMergedObj == null) {
            // create object
            nextMergedObj = {};
            mergedObj[key] = nextMergedObj;
          }
          if (nextPartialObj == null) {
            // create object
            nextPartialObj = {};
            partialObj[key] = nextPartialObj;
          }
          mergedObj = nextMergedObj;
          partialObj = nextPartialObj;
        } else {
          // set value
          mergedObj[key] = partialObj[key];
        }
      }
    }
    // merge setPaths
    (merged as PartialNode<T>).setPaths = [...((base as PartialNode<T>).setPaths ?? []), ...partial.setPaths];
    return merged;
  } else {
    return base;
  }
}

/** Resolve the node in the given graph if it's a reference */
export function resolveNode(graph: ReadNodeGraph, node: AnyNodeData | NodeReferenceData): AnyNodeData {
  return isNodeRef(node) ? graph.getOrError(node) : node;
}

/** Get the siblings of the given node (of the same node type) */
export function getSiblings(graph: ReadNodeGraph, node: AnyNodeData): AnyNodeData[] {
  if (node.parentPtr == null) return [];
  return graph.getChildren(node.parentPtr, node.metatype as unknown as NodeType);
}

/** Get the previous sibling of the given node */
export function getPreviousSibling(graph: ReadNodeGraph, node: AnyNodeData): AnyNodeData | null {
  const siblings = getSiblings(graph, node);
  const index = siblings.findIndex((sibling) => sibling.id == node.id);
  return index > 0 ? siblings[index - 1] : null;
}

/** Get the next sibling of the given node */
export function getNextSibling(graph: ReadNodeGraph, node: AnyNodeData): AnyNodeData | null {
  const siblings = getSiblings(graph, node);
  const index = siblings.findIndex((sibling) => sibling.id == node.id);
  return index < siblings.length - 1 ? siblings[index + 1] : null;
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
  includes: (node: NodeTypeMapping[T]) => boolean;
  includesChildren: (node: NodeTypeMapping[T]) => boolean;
  watchSource?: WatchSource<any>;
}): { items: Ref<NodeTreeItem<T>[]>; trigger: () => void } {
  type NodeT = NodeTypeMapping[T];
  type ItemT = NodeTreeItem<T>;

  const { graph, rootPtr, nodeTypes, isExpanded, includes, includesChildren } = walk;

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
      const children = nodeTypes.value.flatMap((nodeType) => graph.getChildren(node, nodeType)).filter(includes);
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
          if (includesChildren(child)) walkDescendants(child, depth + 1);
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

type NodeSuperGraphCallback = (event: "miss" | "hit" | "sub" | "unsub", key: TypedNodeKey<any>, callback: any) => void;

/**
 * A supergraph composed of multiple subgraphs from different connections.
 * NOTE :Architecture: maybe supergraph should have an index of all nodes in all graphs for fast lookup?
 */
export class NodeSuperGraph {
  connections: Ref<ConnectionBase<any, any>[]>;
  private connectionsByNodeType: Ref<{ [nodeType: number]: ConnectionBase<any, any>[] }>;
  private subs: NodeSuperGraphCallback[] = [];

  constructor(connections: Ref<ConnectionBase<any, any>[]>) {
    this.connections = connections;
    this.connectionsByNodeType = computed(() => {
      const connectionsByNodeType: { [nodeType: number]: ConnectionBase<any, any>[] } = {};
      for (const connection of connections.value) {
        for (const nodeType of connection.nodeTypes) {
          if (!connectionsByNodeType[nodeType]) {
            connectionsByNodeType[nodeType] = [];
          }
          connectionsByNodeType[nodeType].push(connection);
        }
      }
      return connectionsByNodeType;
    });
  }

  subscribeEvent(callback: NodeSuperGraphCallback): () => void {
    this.subs.push(callback);
    return () => {
      const index = this.subs.indexOf(callback);
      if (index >= 0) this.subs.splice(index, 1);
    };
  }

  get liveGraphs(): ReadNodeGraph[] {
    return this.connections.value
      .filter(
        (connection) =>
          connection.result.value != null && "graphComposite" in connection.result.value && connection.meta.live,
      )
      .map((connection) => connection.result.value.graphComposite as ReadNodeGraph);
  }

  getConnectionsFor(...nodeTypes: NodeType[]): ConnectionBase<any, any>[] {
    if (nodeTypes.length == 0) {
      return this.connections.value;
    } else if (nodeTypes.length == 1) {
      return this.connectionsByNodeType.value[nodeTypes[0]] ?? [];
    } else {
      return this.connections.value.filter((connection) =>
        nodeTypes.every((nodeType) => connection.nodeTypes.has(nodeType)),
      );
    }
  }

  /**
   * Gets a node from the supergraph.
   */
  get<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] | null {
    const connections = key.nodeType != null ? this.getConnectionsFor(key.nodeType) : this.connections.value;
    for (const connection of connections) {
      if (connection.result.value != null && "graphComposite" in connection.result.value && connection.meta.live) {
        const graph = connection.result.value.graphComposite as ReadNodeGraph;
        const node = graph.get(key);
        if (node != null) return node;
      }
    }
    return null;
  }

  /** Gets a node from the supergraph or throws an error if it's not found. */
  getOrError<T extends NodeType>(key: NodeKey<T>): NodeTypeMapping[T] {
    const node = this.get(key);
    if (node == null) throw new Error(`node not found: ${describeNode(key)}`);
    return node;
  }

  /**
   * Gets multiple nodes from the supergraph.
   */
  getMany<T extends NodeType>(keys: NodeKey<T>[]): NodeTypeMapping[T][] {
    return keys.map((key) => this.getOrError(key));
  }

  /**
   * Gets multiple nodes from the supergraph maybe.
   */
  getManyMaybe<T extends NodeType>(keys: NodeKey<T>[]): NodeTypeMapping[T][] {
    return keys.map((key) => this.get(key)).filter((node) => node != null);
  }

  /**
   * Gets a link for many nodes. Only returns nodes for the first matching link.
   */
  getLinkMany<T extends NodeType>(
    keys: TypedNodeKey<T>[],
  ): {
    nodes: NodeTypeMapping[T][];
    graph: ReadNodeGraph;
    connection: ConnectionBase<any, any>;
  } | null {
    for (const key of keys) {
      const link = this.getLink(key);
      if (link != null) {
        const nodes = keys.map((key) => link.graph.get(key)).filter((node) => node != null) as NodeTypeMapping[T][];
        return { nodes, graph: link.graph, connection: link.connection };
      }
    }
    return null;
  }

  /**
   * Gets the source connection/graph from the supergraph.
   */
  getLink<T extends NodeType>(
    key: TypedNodeKey<T>,
  ): {
    node: NodeTypeMapping[T];
    graph: ReadNodeGraph;
    connection: ConnectionBase<any, any>;
  } | null {
    for (const connection of this.getConnectionsFor(key.nodeType)) {
      if (connection.result.value != null && "graphComposite" in connection.result.value && connection.meta.live) {
        const graph = connection.result.value.graphComposite as ReadNodeGraph;
        const node = graph.get(key);
        if (node != null) return { node, graph, connection };
      }
    }
    return null;
  }

  /**
   * Gets the source connection/graph from the supergraph or throws an error if it's not found.
   */
  getLinkOrError<T extends NodeType>(
    key: TypedNodeKey<T>,
  ): {
    node: NodeTypeMapping[T];
    graph: ReadNodeGraph;
    connection: ConnectionBase<any, any>;
  } {
    const link = this.getLink(key);
    if (link == null) throw new Error(`no graph found for ${describeNode(key)}`);
    return link;
  }

  /**
   * Subscribe to any change in the given key
   * Unlike ReadNodeGraph, we only subscribe to the first graph containing the node if it exists (otherwise all).
   */
  subscribe<T extends NodeType>(key: TypedNodeKey<T>, callback: NodeGraphCallback): () => void {
    const subs: Array<() => void> = [];
    const unsub = () => {
      subs.forEach((sub) => sub());
      subs.length = 0;
    };
    const update = () => {
      unsub();
      // check all connections for the node
      let found = false;
      for (const connection of this.getConnectionsFor(key.nodeType)) {
        if (connection.result.value != null && "graphComposite" in connection.result.value && connection.meta.live) {
          const graph = connection.result.value.graphComposite as ReadNodeGraph;
          const node = graph.get(key);
          if (node != null) {
            subs.push(
              graph.subscribe(key, () => {
                callback();
                if (graph.get(key) == null) {
                  update(); // only update subscription if node is no longer in graph
                }
              }),
            );
            found = true;
            break;
          }
        }
      }

      // notify
      const e = found ? "hit" : "miss";
      this.subs.forEach((sub) => sub(e, key, callback));

      // subscribe to all graphs and graphs list
      if (!found) {
        for (const connection of this.getConnectionsFor(key.nodeType)) {
          if (connection.result.value != null && "graphComposite" in connection.result.value && connection.meta.live) {
            const graph = connection.result.value.graphComposite as ReadNodeGraph;
            subs.push(graph.subscribe(key, () => (callback(), update())));
          }
          subs.push(watch(connection.result, () => (callback(), update()), { flush: "sync" }));
        }
        subs.push(watch(this.connections, () => (callback(), update()), { flush: "sync" }));
      }
    };
    update();

    this.subs.forEach((sub) => sub("sub", key, callback));
    return () => {
      unsub();
      this.subs.forEach((sub) => sub("unsub", key, callback));
    };
  }

  /**
   * Subscribes to the given key until it is found
   */
  subscribeUntilFound<T extends NodeType>(key: TypedNodeKey<T>, callback: NodeGraphCallback): () => void {
    let unsub: (() => void) | undefined;
    const onFound = () => {
      if (this.get(key) != null) {
        callback();
        if (unsub) {
          unsub();
          unsub = undefined;
        }
      }
    };
    unsub = this.subscribe(key, onFound);
    onFound(); // Check immediately in case node already exists
    return () => {
      if (unsub) unsub();
    };
  }

  /**
   * Gets a reactive reference to the current node with that key
   */
  getRef<T extends NodeType>(key: MaybeRef<TypedNodeKey<T> | null | undefined>): Ref<NodeTypeMapping[T] | null> {
    const keyRef = toRef(key) as Ref<TypedNodeKey<T>>;
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

  /**
   * Gets a reactive reference to multiple nodes from the supergraph.
   */
  getManyRef<T extends NodeType>(keys: MaybeRef<TypedNodeKey<T>[] | null | undefined>): Ref<NodeTypeMapping[T][]> {
    const keysRef = toRef(keys) as Ref<TypedNodeKey<T>[] | null | undefined>;
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
      if (keysRef.value) {
        keysRef.value.filter((k) => k != null).forEach((key) => subs.push(this.subscribe(key, trigger)));
      }
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watchValue(keysRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return ref;
  }

  /**
   * Gets a reactive reference to the source for a node with that key
   */
  getLinkRef<T extends NodeType>(
    key: MaybeRef<TypedNodeKey<T> | null | undefined>,
  ): {
    node: Ref<NodeTypeMapping[T] | null>;
    graph: Ref<ReadNodeGraph | null>;
    connection: Ref<ConnectionBase<any, any> | null>;
  } {
    const keyRef = toRef(key) as Ref<TypedNodeKey<T>>;
    let sub: (() => void) | null = null;
    const unsub = () => (sub != null ? (sub(), (sub = null)) : null);
    const get = () => (keyRef.value != null ? this.getLink(keyRef.value) : null);
    const update = () => {
      unsub();
      if (keyRef.value != null) sub = this.subscribe(keyRef.value, trigger);
    };

    const { ref, trigger } = manualSubRef(get, unsub);
    watch(keyRef, () => (update(), trigger()));
    update();
    tryOnBeforeUnmount(unsub);
    return {
      node: computed(() => ref.value?.node ?? null),
      graph: computed(() => ref.value?.graph ?? null),
      connection: computed(() => ref.value?.connection ?? null),
    };
  }
}
