import { getCachedGraphClient, HUMANIZED_OPERATION_STATUS } from "@/proto/services";
import {
  AccessType,
  BlockProperty,
  ChangeCategory,
  CommitTransactionRequest,
  EditType,
  GraphScopeData,
  LogData,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  Struct as ProtoStruct,
  TextData,
  Timestamp,
  type AnyNodeData,
  type EditData,
  type NodeTypeMapping,
  type PropertyInfo,
} from "@/proto/wire";
import {
  describeEdit,
  describeNode,
  EMPTY_SCOPE,
  fillDefaultObject,
  makeDefaultObject,
  makeScope,
  nodeReference,
  toPlainNodeRef,
  unwrapSomeNode,
  wrapSomeNode,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { nonce, origin, userOrNullPtr, userPtr } from "@/system/client";
import { type ReadNodeGraph, type WriteNodeGraph } from "@/language/graph";
import { makeIcon } from "@/ui/icon";
import { makeNode } from "@/language/node";
import { toaster } from "@/ui/toast";
import { unpackBuiltinObject, type JsonValue } from "@/language/value";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { toValueRef } from "@/utils/ref";
import { uuidt } from "@/utils/uuidt";
import type { RpcError } from "grpc-web";
import { nextTick, ref, shallowRef, toRef, triggerRef, watch, type MaybeRef, type Ref } from "vue";
import { toCamelName } from "@/language/const";

export type DebounceLevel = "tick" | "short" | "long";
const DEBOUNCE_LEVELS: Record<"short" | "long", number> = {
  short: 500,
  long: 2000,
};

const IMPLICIT_PROPERTIES = [
  "metatype",
  "id",
  "ck",
  "createdAt",
  "createdEpoch",
  "createdByPtr",
  "updatedAt",
  "updatedEpoch",
  "updatedByPtr",
  "revision",
];
const IMPLICIT_UPDATE_PROPERTIES_IDS = ["updatedAt", "updatedEpoch", "updatedByPtr", "revision"].map(
  (p) => BlockProperty[p as any] as unknown as number,
);
const NONCE_POSTFIX = nonce.replace("-", "").slice(0, 16);

function newEditId(): string {
  return uuidt({ nonce: NONCE_POSTFIX });
}

function newTransactionId(): string {
  return uuidt({ nonce: NONCE_POSTFIX });
}

/** A transaction on the Bench state graph. */
export type Transaction = {
  readonly scope: GraphScopeData;
  readonly id: string;
  readonly edits: EditData[];
  describeSelf(): string;

  /** Adds an externally created edit */
  addEdit(edit: EditData): void;
  /** Gets the sub tx for a specific connection */
  with(meta: { connectionId?: number; change?: ChangeIn; category?: ChangeCategory }): Transaction;

  /** Create a new node */
  create<T extends NodeType>(
    node: { metatype: T | ObjectType } & Partial<Omit<NodeTypeMapping[T], "metatype">>,
  ): NodeTypeMapping[T];
  /** Create or update all properties in the node */
  upsert(node: AnyNodeData): void;

  /** Update regular properties in this node. v*/
  update<T extends AnyNodeData>(node: T, update: Partial<T>, options?: { debounce?: DebounceLevel }): void;
  /** Move node between parents (and update it) */
  move<T extends AnyNodeData>(
    node: T,
    update: Partial<T> & { parentPtr: NodeReferenceData },
    options?: { debounce?: DebounceLevel },
  ): void;
  /** Archive node (incl. descendants) */
  archive(node: AnyNodeData): void;
  /** Restore node from archive (incl. descendants) */
  unarchive(node: AnyNodeData): void;

  /** Soft delete node (incl. descendants), marked for later deletion after retention period */
  delete(node: AnyNodeData): void;
  /** Restore node from soft delete */
  restore(node: AnyNodeData): void;
  /** Erase a node and its descendants forever */
  erase(node: AnyNodeData): void;
};

/* Shared state to create multiple TransactionBuilder handles from different connections with same data */
export class TransactionState {
  readonly id: string;
  readonly scope: GraphScopeData;
  readonly benchPtr: TypedNodeReferenceData<NodeType.BENCH> | null;
  readonly edits: EditData[] = [];

  readonly _debouncedUpdates: Record<string, EditData> = {};
  readonly _subs: Array<(edit: EditData, meta: TransactionMeta, debounce: DebounceLevel | null) => void> = [];
  readonly _txByConnectionId: Record<number, TransactionBuilder> = {};

  constructor(id: string, scope: GraphScopeData) {
    this.id = id;
    this.scope = scope;
    this.benchPtr = scope.benchId != null ? nodeReference(NodeType.BENCH, scope.benchId) : null;
  }
}

export type TransactionMeta = {
  connectionId?: number;
  subjectRef?: Ref<NodeReferenceData | null>;
  change?: ChangeIn;
  category?: ChangeCategory;
};
export type ChangeIn = {
  key: string;
  title?: string;
  text?: TextData;
};

/**
 * Build a transaction (maybe for a specific connection/change).
 * We often want to share transaction state across multiple builders with curried info (connectionId, change, category, ...).
 */
export class TransactionBuilder implements TransactionMeta, Transaction {
  public readonly connectionId: number | undefined;
  public readonly subjectRef: Ref<NodeReferenceData | null>;
  public readonly change: ChangeIn | undefined;
  public readonly category: ChangeCategory | undefined;

  state: TransactionState;

  constructor(tx: {
    state: TransactionState;
    connectionId?: number;
    subject: MaybeRef<NodeReferenceData | null>;
    change?: ChangeIn;
    category?: ChangeCategory;
  }) {
    this.state = tx.state;
    this.connectionId = tx.connectionId;
    this.subjectRef = toRef(tx.subject);
    this.change = tx.change;
    this.category = tx.category;
  }

  get id() {
    return this.state.id;
  }

  get scope(): GraphScopeData {
    return this.state.scope;
  }

  get edits(): EditData[] {
    return this.state.edits;
  }

  get subject(): NodeReferenceData {
    if (!this.subjectRef.value) throw new Error("subject not set");
    return this.subjectRef.value;
  }

  with(meta: { connectionId?: number; change?: ChangeIn; category?: ChangeCategory }): Transaction {
    if (meta.connectionId == this.connectionId && meta.change == this.change && meta.category == this.category) {
      return this; // no change
    }

    let base: TransactionBuilder = this;

    // cache by connection id
    if (meta.connectionId != null) {
      if (this.state._txByConnectionId[meta.connectionId] == null) {
        this.state._txByConnectionId[meta.connectionId] = new TransactionBuilder({
          state: this.state,
          subject: this.subjectRef,
          ...meta,
        });
      }
      base = this.state._txByConnectionId[meta.connectionId];
    }

    // and split if change/category is specified
    if (meta.change != null || meta.category != null) {
      return new TransactionBuilder({
        state: this.state,
        connectionId: base.connectionId,
        subject: base.subjectRef,
        change: meta.change ?? base.change,
        category: meta.category ?? base.category,
      });
    } else {
      return base;
    }
  }

  describeSelf(): string {
    return `Transaction(${this.id}, ${this.state.edits.length} edits)`;
  }

  onEdit(sub: (edit: EditData, meta: TransactionMeta, debounce: DebounceLevel | null) => void): () => void {
    this.state._subs.push(sub);
    return () => {
      const idx = this.state._subs.indexOf(sub);
      if (idx >= 0) this.state._subs.splice(idx, 1);
    };
  }

  getScope(node: AnyNodeData): GraphScopeData {
    const allProperties = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
    const benchId = (node as any).benchPtr?.id ?? this.scope.benchId;
    const packageId = (node as any).packagePtr?.id ?? this.scope.packageId;
    if ("packagePtr" in allProperties && packageId == null)
      throw new Error(`missing packagePtr in ${describeNode(node)}`);
    return makeScope({ benchId, packageId });
  }

  checkInScope(node: AnyNodeData) {
    if (this.scope.benchId != null && (!("benchPtr" in node) || node.benchPtr?.id != this.scope.benchId)) {
      throw new Error(`node from other bench: ${describeNode(node)} != ${this.scope.benchId}`);
    }
    if (this.scope.packageId != null && (!("packagePtr" in node) || node.packagePtr?.id != this.scope.packageId)) {
      throw new Error(`node from other package: ${describeNode(node)} != ${this.scope.packageId}`);
    }
  }

  /** Adds a simple (non-update) edit */
  _addSimpleEdit(
    editType:
      | EditType.CREATE
      | EditType.UPSERT
      | EditType.ARCHIVE
      | EditType.UNARCHIVE
      | EditType.DELETE
      | EditType.RESTORE
      | EditType.ERASE,
    node: AnyNodeData,
    debounce: DebounceLevel | null,
  ) {
    this.checkInScope(node);

    // pack 'old' and 'new' node delta :EditData
    let newNode: AnyNodeData | undefined = undefined;
    let oldNode: AnyNodeData | undefined = undefined;
    if (editType == EditType.CREATE || editType == EditType.UPSERT) {
      newNode = node;
    } else if (editType == EditType.ERASE || editType == EditType.ARCHIVE || editType == EditType.DELETE) {
      // clear deletedAt/archivedAt
      if (node.deletedAt != null || node.archivedAt != null) {
        oldNode = { ...node, deletedAt: undefined, archivedAt: undefined };
      } else {
        oldNode = node;
      }
    } else if (editType == EditType.UNARCHIVE) {
      // remember old 'archived_at' in old node, put full restored node in new node
      oldNode = makeDefaultObject({ metatype: node.metatype as any, archivedAt: node.archivedAt }) as AnyNodeData;
    } else if (editType == EditType.RESTORE) {
      // remember old 'deleted_at' in old node, put full restored node in new node
      oldNode = makeDefaultObject({ metatype: node.metatype as any, deletedAt: node.deletedAt }) as AnyNodeData;
    }

    // make edit & notify
    const edit: EditData = {
      metatype: ObjectType.EDIT,
      id: newEditId(),
      type: editType,
      nodePtr: toPlainNodeRef(node),
      scope: this.getScope(node),
      oldNodePartial: oldNode != null ? wrapSomeNode(oldNode) : undefined,
      newNodePartial: newNode != null ? wrapSomeNode(newNode) : undefined,
      properties: [],
      origin: origin.value,
      subjectPtr: this.subject,
      category: this.category,
      editedAt: Timestamp.now(),
    };
    this.state.edits.push(edit);
    this._notifyEdit(edit, debounce);
    return edit;
  }

  _notifyEdit(edit: EditData, debounce: DebounceLevel | null) {
    for (const sub of this.state._subs) {
      sub(edit, this, debounce);
    }
  }

  addEdit(edit: EditData): void {
    this.state.edits.push(edit);
    this._notifyEdit(edit, null);
  }

  create<T extends NodeType>(
    nodeIn: { metatype: T | ObjectType } & Partial<Omit<NodeTypeMapping[NodeType], "metatype">>,
  ): NodeTypeMapping[T] {
    // fill in scope
    const properties = PROPERTY_ENUM_BY_TYPE[nodeIn.metatype as unknown as ObjectType];
    if (properties == null) {
      throw new Error(`missing properties for ${nodeIn.metatype}`);
    } else if ("packagePtr" in properties && (nodeIn as any).packagePtr == null) {
      throw new Error(`missing packagePtr in ${describeNode(nodeIn)}`); // can't infer package
    } else if ("benchPtr" in properties) {
      if ((nodeIn as any).benchPtr == null) {
        if (this.state.benchPtr == null) {
          throw new Error(`missing benchPtr to infer bench for ${describeNode(nodeIn)}`);
        }
        (nodeIn as any).benchPtr = this.state.benchPtr; // infer bench
      }
    }

    // create node
    // NOTE :Cleanup: why doesn't makeNode typecheck properly here?
    const node: NodeTypeMapping[T] =
      nodeIn.id == null ? makeNode(nodeIn as any) : (nodeIn as unknown as NodeTypeMapping[T]);

    this._addSimpleEdit(EditType.CREATE, node, null);
    return node as NodeTypeMapping[T];
  }

  upsert(node: AnyNodeData) {
    this._addSimpleEdit(EditType.UPSERT, { ...node }, null);
  }

  _doUpdate<T extends AnyNodeData>(
    editType: EditType.UPDATE | EditType.MOVE,
    node: T,
    update: Partial<T>,
    options?: { debounce?: DebounceLevel },
  ) {
    this.checkInScope(node);

    const propertiesEnum = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
    const allProperties = PROPERTY_INFOS_BY_TYPE[node.metatype]!;

    // collect properties
    const properties: PropertyInfo[] = [];
    for (const propName of Object.keys(update)) {
      if (IMPLICIT_PROPERTIES.includes(propName)) {
        throw new Error(`cannot update constant property for ${describeNode(node)}: ${propName}`);
      }
      const propId = propertiesEnum[propName as any];
      const propInfo = allProperties[propId];
      if (propInfo == null) {
        throw new Error(`missing property info for ${describeNode(node)}: ${propName}/${propId}`);
      }
      properties.push(propInfo);
    }

    if (!options?.debounce || !this.state._debouncedUpdates[node.id]) {
      // create new edit
      const oldNode = { ...node } as T;
      const newNode = { ...oldNode, ...update } as T;
      for (const prop of properties) {
        const propName = propertiesEnum[prop.id];
        (oldNode as any)[propName] = (node as any)[propName];
        (newNode as any)[propName] = (update as any)[propName];
      }
      const edit: EditData = {
        metatype: ObjectType.EDIT,
        id: newEditId(),
        type: editType,
        nodePtr: toPlainNodeRef(node),
        scope: this.getScope(node),
        properties: properties.map((p) => p.id),
        oldNodePartial: wrapSomeNode(oldNode),
        newNodePartial: wrapSomeNode(newNode),
        origin: origin.value,
        subjectPtr: this.subject,
        category: this.category,
        editedAt: Timestamp.now(),
      };
      if (options?.debounce) {
        this.state._debouncedUpdates[node.id] = edit;
      }
      this.state.edits.push(edit);
      this._notifyEdit(edit, options?.debounce ?? null);
    } else {
      // merge into existing edit & notify directly :DebouncedUpdate
      const edit = this.state._debouncedUpdates[node.id];
      if (edit.oldNodePartial == null || edit.newNodePartial == null) {
        throw new Error(`missing old/new node in debounced edit: ${describeEdit(edit)}`);
      }
      const oldNodePartial = unwrapSomeNode(edit.oldNodePartial);
      const newNodePartial = unwrapSomeNode(edit.newNodePartial);
      for (const prop of properties) {
        const propName = propertiesEnum[prop.id];
        // add to Edit.properties if not there @yet
        if (!edit.properties.includes(prop.id)) {
          // and old value since it doesn't already exist
          edit.properties.push(prop.id);
          (oldNodePartial as any)[propName] = (node as any)[propName];
        }
        // and update new value
        (newNodePartial as any)[propName] = (update as any)[propName];
      }
      // coalesce successive move/update into move edit
      if (editType == EditType.MOVE && edit.type != EditType.MOVE) {
        edit.type = EditType.MOVE;
      }
      this._notifyEdit(edit, options?.debounce);
    }
  }

  update<T extends AnyNodeData>(node: T, update: Partial<T>, options?: { debounce?: DebounceLevel }) {
    this._doUpdate(EditType.UPDATE, node, update, options);
  }

  move<T extends AnyNodeData>(
    node: T,
    update: Partial<T> & { parentPtr: NodeReferenceData },
    options?: { debounce?: DebounceLevel },
  ) {
    this._doUpdate(EditType.MOVE, node, update, options);
  }

  archive(node: AnyNodeData) {
    this._addSimpleEdit(EditType.ARCHIVE, { ...node }, null);
  }

  unarchive(node: AnyNodeData) {
    this._addSimpleEdit(EditType.UNARCHIVE, { ...node, archivedAt: undefined }, null);
  }

  delete(node: AnyNodeData) {
    this._addSimpleEdit(EditType.DELETE, { ...node }, null);
  }

  restore(node: AnyNodeData) {
    this._addSimpleEdit(EditType.RESTORE, { ...node, deletedAt: undefined }, null);
  }

  erase(node: AnyNodeData) {
    this._addSimpleEdit(EditType.ERASE, { ...node }, null);
  }
}

export function packProtoJson(value: JsonValue | null | undefined): ProtoStruct {
  return ProtoStruct.fromJson(value ?? null);
}

export function unpackProtoJson(value: ProtoStruct): JsonValue {
  return ProtoStruct.toJson(value);
}

/**
 * Applies the edits to the graph (in place!).
 * If a base graph is given, this graph is assumed to be an overlay.
 */
export function editGraph(
  graph: ReadNodeGraph & WriteNodeGraph,
  edits: EditData[],
  options?: { base?: ReadNodeGraph },
) {
  for (const edit of edits) {
    const nodeType = edit.nodePtr!.type;
    if (
      edit.type == EditType.CREATE ||
      edit.type == EditType.UPSERT ||
      ((edit.type == EditType.UNARCHIVE || edit.type == EditType.RESTORE) && !graph.has(edit.nodePtr!))
    ) {
      // add
      let newNode: AnyNodeData;
      if (edit.type == EditType.CREATE || edit.type == EditType.UPSERT) {
        if (edit.newNodePartial == null) throw new Error(`missing newNodePacked in edit: ${describeEdit(edit)}`);
        newNode = unwrapSomeNode(edit.newNodePartial);
      } else {
        if (edit.oldNodePartial == null) throw new Error(`missing oldNodePacked in edit: ${describeEdit(edit)}`);
        newNode = unwrapSomeNode(edit.oldNodePartial);
        if (edit.type == EditType.UNARCHIVE) {
          newNode = { ...newNode, archivedAt: undefined };
        } else if (edit.type == EditType.RESTORE) {
          newNode = { ...newNode, deletedAt: undefined };
        }
      }
      // implicit metadata
      newNode.createdAt = newNode.updatedAt = edit.editedAt;
      if (edit.epoch != null && "createdEpoch" in newNode && "updatedEpoch" in newNode) {
        newNode.createdEpoch = newNode.updatedEpoch = edit.epoch;
      }
      newNode.createdByPtr = newNode.updatedByPtr = edit.subjectPtr;
      if (edit.type == EditType.CREATE || !graph.has(newNode)) {
        graph.add(newNode);
      } else {
        graph.update(newNode);
      }
    } else if (edit.type == EditType.ERASE && !(options?.base && !graph.has(edit.nodePtr!))) {
      // remove
      const oldNode = graph.get(edit.nodePtr!);
      if (!oldNode) {
        throw new Error(`missing node for delete: ${describeNode(edit.nodePtr!)} in ${graph.describeSelf()}`);
      }
      graph.remove(oldNode);
    } else {
      // update
      let updatedNode: AnyNodeData | null;
      if (edit.type == EditType.UNARCHIVE || edit.type == EditType.RESTORE) {
        if (edit.oldNodePartial == null)
          throw new Error(`missing old node in edit: ${describeEdit(edit)} in ${graph.describeSelf()}`);
        updatedNode = unwrapSomeNode(edit.oldNodePartial);
        updatedNode = { ...updatedNode, archivedAt: undefined };
      } else {
        updatedNode = graph.get(edit.nodePtr!);
        if (!updatedNode && options?.base) {
          updatedNode = options.base.get(edit.nodePtr!);
        }
        if (!updatedNode) {
          throw new Error(`missing node for update: ${describeNode(edit.nodePtr!)} in ${graph.describeSelf()}`);
        }
      }
      updatedNode = { ...updatedNode }; // copy

      // directly edited properties
      if (edit.type == EditType.UPDATE || edit.type == EditType.MOVE) {
        if (edit.newNodePartial == null)
          throw new Error(`missing new node in edit: ${describeEdit(edit)} in ${graph.describeSelf()}`);
        const newNode = unwrapSomeNode(edit.newNodePartial);
        const propertyEnum = NODE_PROPERTY_ENUM_BY_TYPE[nodeType]!;
        const allProperties = PROPERTY_INFOS_BY_TYPE[nodeType]!;
        for (const propId of edit.properties) {
          const prop = allProperties[propId];
          if (!prop) throw new Error(`missing property info for ${nodeType}: ${propId}`);
          const propName = propertyEnum[propId];
          const newValue = (newNode as any)[propName];
          (updatedNode as any)[propName] = newValue;
        }
      }

      // implicit metadata
      const extraImplicitProperties: number[] = [];
      updatedNode.updatedAt = edit.editedAt;
      if (edit.epoch != null && "updatedEpoch" in updatedNode) {
        updatedNode.updatedEpoch = edit.epoch;
      }
      updatedNode.updatedByPtr = edit.subjectPtr;
      updatedNode.revision = edit.revision ?? BigInt(-1);
      if (edit.type == EditType.ARCHIVE) {
        updatedNode.archivedAt = edit.editedAt;
        extraImplicitProperties.push(BlockProperty.archivedAt);
      } else if (edit.type == EditType.UNARCHIVE) {
        updatedNode.archivedAt = undefined;
        extraImplicitProperties.push(BlockProperty.archivedAt);
      } else if (edit.type == EditType.DELETE || edit.type == EditType.ERASE) {
        // (we handle DELETE here for overlays)
        updatedNode.deletedAt = edit.editedAt;
        extraImplicitProperties.push(BlockProperty.deletedAt);
      } else if (edit.type == EditType.RESTORE) {
        updatedNode.deletedAt = undefined;
        extraImplicitProperties.push(BlockProperty.deletedAt);
      }

      // extend setProperties for overlay
      if (options?.base != null) {
        if ((updatedNode.setProperties?.length ?? 0) == 0) {
          updatedNode.setProperties = [
            ...IMPLICIT_UPDATE_PROPERTIES_IDS,
            ...edit.properties,
            ...extraImplicitProperties,
          ];
        } else {
          for (const propId of [...edit.properties, ...extraImplicitProperties]) {
            if (!updatedNode.setProperties.includes(propId)) updatedNode.setProperties.push(propId);
          }
        }
      }

      graph.update(updatedNode);
    }
  }
}

type CommitFailure = {
  id: string;
  edits: EditData[];
  error: RpcError;
};
type PendingCallback = (
  event:
    | { type: "add"; meta: TransactionMeta; edits: EditData[]; debounce: DebounceLevel | null }
    | { type: "reset"; meta: TransactionMeta; edits: EditData[] },
) => void;
type CommittedCallback = (edits: EditData[]) => void;

/**
 * A transaction buffer provides Transactions and applies them to the graph.
 */
export interface TransactionBuffer {
  readonly id: number;
  /** Current active Transaction. */
  readonly tx: Transaction;
  /** Retryable commits in case of error (for debugging).  */
  readonly failedCommits?: Readonly<Ref<Record<string, CommitFailure>>>;

  /** Commits the current transaction. */
  commit(): void | Promise<void>;
  /** Resets the current transaction and overlay. */
  reset(): void | Promise<void>;
  /** Accepts the given edits from an external source (does not trigger committed) */
  accept(edits: EditData[]): void;
  /** Subscribes to *pending* edits from this buffer */
  onPending(sub: PendingCallback): () => void;
  /** Subscribes to *committed* edits from this buffer */
  onCommitted(sub: CommittedCallback): () => void;
  /** Force retries the given commit (for debugging) */
  retry?(id: string): Promise<void>;

  /** Whether there are any pending uncommitted edits  */
  get isDirty(): boolean;
  /** Whether there are any active commits */
  get isCommitting(): boolean;

  /** Whether transactions are currently processed (for debugging). */
  readonly isPaused: Readonly<Ref<boolean>>;
  /** Toggle automatic flushing (for debugging). */
  togglePaused(): void;
}

/**
 * Applies transactions immediately to the graph.
 */
export class ImmediateTransactionBuffer implements TransactionBuffer {
  public readonly id: number;
  public readonly scope: GraphScopeData;
  public readonly graph: ReadNodeGraph & WriteNodeGraph;
  public readonly isPaused: Ref<boolean> = ref(false);
  private _committedSubs: Array<CommittedCallback> = [];
  private _currentTx: TransactionBuilder | null = null; // always keep a single transaction

  constructor(id: number, scope: GraphScopeData, graph: ReadNodeGraph & WriteNodeGraph) {
    this.id = id;
    this.scope = scope;
    this.graph = graph;
    this.reset();
  }

  get tx(): Transaction {
    return this._currentTx!; // set in constructor
  }

  commit() {
    // nothing to do
  }

  reset() {
    // NOTE: we default to null user pointer in immediate transaction buffer since it's only used locally
    //  and we need some 'subject' to create edits (even when not connected to a 'real' remote graph)
    const newTx = new TransactionBuilder({
      subject: userOrNullPtr,
      state: new TransactionState(uuidt({ nonce: NONCE_POSTFIX }), this.scope),
    });
    // immediately apply and reset the transaction
    newTx.onEdit((edit) => {
      if (this._currentTx !== newTx) throw new Error("transaction is closed");
      // apply edit directly
      editGraph(this.graph, [edit]);
      // notify
      this._committedSubs.forEach((sub) => sub([edit]));
      // 'reset'
      newTx.state.edits.length = 0;
    });
    this._currentTx = newTx;
  }

  accept(edits: EditData[]) {
    // nothing to do
  }

  onPending(sub: PendingCallback): () => void {
    // nothing to do
    return () => {};
  }

  onCommitted(sub: CommittedCallback): () => void {
    this._committedSubs.push(sub);
    return () => {
      const idx = this._committedSubs.indexOf(sub);
      if (idx >= 0) this._committedSubs.splice(idx, 1);
    };
  }

  togglePaused() {
    // nothing to do
  }

  get isDirty() {
    return false;
  }

  get isCommitting() {
    return false;
  }
}

/**
 * A buffer with one active & one pending transaction that is committed to a remote client.
 */
export class RemoteTransactionBuffer implements TransactionBuffer {
  public readonly id: number;
  public readonly scope: GraphScopeData;
  public readonly isPaused: Ref<boolean> = ref(false);
  private pendingSubs: Array<PendingCallback> = [];
  private committedSubs: Array<CommittedCallback> = [];
  private currentTx: Transaction | null;
  private pendingTx: Transaction | null;
  private pendingEditsById: Record<string, EditData> = {};
  private pendingConnectionByEditId: Record<string, number> = {};
  failedCommits: Ref<Record<string, CommitFailure>> = shallowRef({});

  constructor(id: number, scope: GraphScopeData) {
    this.id = id;
    this.scope = scope;
    this.currentTx = null;
    this.pendingTx = null;
    this.reset();
  }

  get tx(): Transaction {
    if (this.currentTx == null) throw new Error("no active transaction");
    return this.currentTx;
  }

  async commit() {
    if (this.currentTx == null) throw new Error("no active transaction");
    if (this.pendingTx != null) throw new Error(`transaction ${this.pendingTx.id} is already committing`);
    try {
      log.trace("transaction.commit", { scope: this.scope, id: this.currentTx.id, edits: this.currentTx.edits });

      const client = getCachedGraphClient(this.scope);
      if (client == null) throw new Error(`no client for ${this.scope}`);

      // swap
      const edits = this.currentTx.edits;
      this.pendingTx = this.currentTx;
      this.currentTx = this._makeCurrentTx();

      // commit
      const {
        response: { epoch, revisions },
      } = await client.commitTransaction(
        { edits, id: this.pendingTx.id, scope: this.scope },
        {
          suppressErrors: true,
          retry: {
            amendRetry: (request: CommitTransactionRequest) => {
              // swap again to include new pending edits in next attempt
              const pendingTx = this.currentTx;
              if (pendingTx == null) throw new Error("no current transaction");
              const newEdits = pendingTx.edits;
              request = { ...request, id: pendingTx.id, edits: [...request.edits, ...newEdits] };
              this.currentTx = this._makeCurrentTx();
              return request;
            },
          },
        },
      );
      for (let i = 0; i < edits.length; i++) {
        edits[i].revision = revisions[i];
      }

      // notify on success
      // (we do not directly edit state on success, when/how/which edits to accept is up to the caller)
      this.committedSubs.forEach((sub) => sub(edits));
    } catch (error) {
      // failed
      const fail: CommitFailure = { id: this.pendingTx!.id, edits: this.pendingTx!.edits, error: error as RpcError };
      this.failedCommits.value[fail.id] = fail;
      triggerRef(this.failedCommits);

      // rollback
      log.error("transaction.commit.error", { scope: this.scope, error });
      toaster.error({
        title: HUMANIZED_OPERATION_STATUS[(error as RpcError).code] ?? "Synchronization error",
        text: `Saving ${this.pendingTx?.edits.length ?? 0} edits failed: ${IS_DEV ? (error as Error).message : (error as RpcError).code}`,
        actions: [
          {
            title: "Retry",
            icon: makeIcon("fas fa-redo"),
            action: () => this.retry(fail.id),
          },
        ],
      });
      this.reset();
    } finally {
      this.pendingTx = null;
    }
  }

  async reset() {
    this.currentTx = this._makeCurrentTx();
    this.pendingEditsById = {};
    this.pendingConnectionByEditId = {};
    this.pendingTx = null;
    this.pendingSubs.forEach((sub) => sub({ type: "reset", meta: {}, edits: [] }));
  }

  async retry(id: string) {
    // re-add the given commit to the current transaction
    const fail = this.failedCommits.value[id];
    if (!fail) throw new Error(`no failed commit with id ${id}`);
    const edits = fail.edits;
    for (const edit of edits) {
      this.currentTx!.addEdit(edit);
    }
  }

  private _makeCurrentTx() {
    const tx = new TransactionBuilder({
      subject: userPtr,
      state: new TransactionState(newTransactionId(), this.scope),
    });
    tx.onEdit((edit, meta, debounce) => {
      if (this.currentTx?.id !== tx.id) throw new Error(`transaction ${tx.describeSelf()} is closed`);
      const pendingConnectionId = this.pendingConnectionByEditId[edit.id];
      if (pendingConnectionId != null && (!meta.connectionId || meta.connectionId != pendingConnectionId))
        // ensure connections don't trample on each others edits since we currently only optimistically overlay
        //  edits from the same connection (see connection)
        throw new Error(`edit ${edit.id} already pending in ${pendingConnectionId}`);
      this.pendingEditsById[edit.id] = edit;
      if (meta.connectionId != null) this.pendingConnectionByEditId[edit.id] = meta.connectionId;
      this.pendingSubs.forEach((sub) => sub({ type: "add", meta, edits: [edit], debounce }));
    });
    return tx;
  }

  accept(edits: EditData[]): void {
    let pendingEditsChanged = false;
    for (const edit of edits) {
      if (this.pendingEditsById[edit.id]) {
        delete this.pendingEditsById[edit.id];
        if (this.pendingConnectionByEditId[edit.id] != null) delete this.pendingConnectionByEditId[edit.id];
        pendingEditsChanged = true;
      }
    }
    if (pendingEditsChanged) {
      // update all pending subscribers (in multiple steps so they get the correct connectionId if we have one)
      const newPendingEdits = Object.values(this.pendingEditsById);
      const newPendingEditsByConnection: Record<number, EditData[]> = {};
      for (const edit of newPendingEdits) {
        const connectionId = this.pendingConnectionByEditId[edit.id] ?? -1;
        if (!newPendingEditsByConnection[connectionId]) newPendingEditsByConnection[connectionId] = [];
        newPendingEditsByConnection[connectionId].push(edit);
      }
      this.pendingSubs.forEach((sub) => {
        sub({ type: "reset", meta: {}, edits: newPendingEditsByConnection[-1] ?? [] });
        for (const connectionId of Object.keys(newPendingEditsByConnection)) {
          sub({
            type: "add",
            meta: { connectionId: parseInt(connectionId) },
            edits: newPendingEditsByConnection[parseInt(connectionId)]!,
            debounce: null,
          });
        }
      });
    }
  }

  onPending(sub: PendingCallback): () => void {
    this.pendingSubs.push(sub);
    return () => {
      const idx = this.pendingSubs.indexOf(sub);
      if (idx >= 0) this.pendingSubs.splice(idx, 1);
    };
  }

  onCommitted(sub: CommittedCallback): () => void {
    this.committedSubs.push(sub);
    return () => {
      const idx = this.committedSubs.indexOf(sub);
      if (idx >= 0) this.committedSubs.splice(idx, 1);
    };
  }

  togglePaused() {
    this.isPaused.value = !this.isPaused.value;
    log.debug("transaction.togglePaused", { scope: this.scope, paused: this.isPaused.value });
    toaster.debug({
      title: this.isPaused.value ? "Buffer paused" : "Buffer resumed",
      text: `Buffer ${this.id} is ${this.isPaused.value ? "pausing transactions" : "resuming transactions"}.`,
      override: `transaction.togglePaused.${this.id}`,
    });
  }

  get isDirty() {
    return this.currentTx != null && this.currentTx.edits.length > 0;
  }

  get isCommitting() {
    return this.pendingTx != null;
  }
}

let bufferId = 0;
export function newBufferId() {
  return bufferId++;
}
const globalTxBuffer: TransactionBuffer = new RemoteTransactionBuffer(newBufferId(), EMPTY_SCOPE);
const txBuffersByBenchId: Ref<Record<string, RemoteTransactionBuffer>> = shallowRef({});

export function getAllTransactionBuffers(): TransactionBuffer[] {
  return [globalTxBuffer, ...Object.values(txBuffersByBenchId.value)];
}

/**
 * Gets the transaction buffer for the given scope (non-exclusively).
 * Currently we maintain one shared buffer per Bench and one for other global nodes.
 * */
export async function getTransactionBuffer(scope: GraphScopeData): Promise<TransactionBuffer> {
  if (scope.benchId) {
    if (!txBuffersByBenchId.value[scope.benchId]) {
      // synchronize so that only one buffer is created per bench even when called concurrently
      if (!txBuffersByBenchId.value[scope.benchId]) {
        const buffer = new RemoteTransactionBuffer(newBufferId(), scope);
        txBuffersByBenchId.value[scope.benchId] = buffer;
        triggerRef(txBuffersByBenchId);
        watchTransactionBuffer(buffer);
      }
    }
    return txBuffersByBenchId.value[scope.benchId];
  } else {
    return globalTxBuffer;
  }
}

/** Commits the transaction buffer on any non-debounced edit, and a commit with delay after any debounced edit  */
function watchTransactionBuffer(buffer: TransactionBuffer) {
  let scheduledCommit = false;
  let scheduledDebouncedCommit: any | null = null;

  // watch pending edit
  buffer.onPending((event) => {
    if (event.type != "add") return; // only react to added edits
    if (scheduledCommit) return; // already wanted
    if (event.debounce != null && event.debounce != "tick") {
      // schedule commit after debounce
      if (scheduledDebouncedCommit != null) clearTimeout(scheduledDebouncedCommit);
      scheduledDebouncedCommit = setTimeout(scheduleCommit, DEBOUNCE_LEVELS[event.debounce]);
    } else {
      // commit on next tick
      scheduleCommit();
    }
  });

  /** Schedules a commit on next tick if not already scheduled */
  function scheduleCommit() {
    if (scheduledCommit) return;
    scheduledCommit = true;
    nextTick(() => {
      scheduledCommit = false;
      if (scheduledDebouncedCommit != null) clearTimeout(scheduledDebouncedCommit);
      if (!buffer.isCommitting && !buffer.isPaused.value) {
        if (buffer.isDirty) buffer.commit();
      } else {
        setTimeout(() => nextTick(scheduleCommit), 50);
      }
    });
  }

  // schedule commit whenever unpaused
  watch(buffer.isPaused, (paused) => {
    if (!paused) scheduleCommit();
  });
}

/** Commits any pending transactions in the current buffers. */
async function flushTransactionBuffers() {
  const buffers = [globalTxBuffer, ...Object.values(txBuffersByBenchId.value)];
  const commitPromises = [];
  for (const tx of buffers) {
    if (tx.isDirty && !tx.isCommitting && !tx.isPaused.value) {
      const ret = tx.commit();
      if (ret instanceof Promise) commitPromises.push(ret);
    }
  }
  await Promise.all(commitPromises);
}

let _setupTransactionManagement = false;
/** Start automatic transaction rotation. */
export function setupTransactionManagement() {
  if (_setupTransactionManagement) return;
  _setupTransactionManagement = true;
  // watch buffers
  watchTransactionBuffer(globalTxBuffer);
  // (the rest is watched on demand)
  // commit on user change
  watch(toValueRef(userPtr), () => flushTransactionBuffers());
  // commit before exit
  window.addEventListener("beforeunload", (e) => flushTransactionBuffers());
}

export const EDIT_TYPES = [
  EditType.CREATE,
  EditType.UPSERT,
  EditType.UPDATE,
  EditType.MOVE,
  EditType.ARCHIVE,
  EditType.UNARCHIVE,
  EditType.DELETE,
  EditType.RESTORE,
  EditType.ERASE,
];
/** Map edit type to inverted edit type */
export const UNDO_EDIT_BY_TYPE: Partial<Record<EditType, EditType>> = {
  [EditType.CREATE]: EditType.DELETE,
  [EditType.UPSERT]: EditType.DELETE,
  [EditType.UPDATE]: EditType.UPDATE,
  [EditType.MOVE]: EditType.MOVE,
  [EditType.ARCHIVE]: EditType.UNARCHIVE,
  [EditType.UNARCHIVE]: EditType.ARCHIVE,
  [EditType.DELETE]: EditType.RESTORE,
  [EditType.RESTORE]: EditType.DELETE,
};

/** Turns a logged edit back into an edit (to redo/undo) */
export function makeEditFromLog(
  log: LogData,
  mode: "redo" | "undo",
  options?: {
    category?: ChangeCategory;
    subjectPtr?: NodeReferenceData;
  },
): EditData {
  // unpack
  if (log.nodePtr == null) throw new Error(`missing node for ${log.type}: ${describeNode(log)}`);
  const nodeType = log.nodePtr.type;
  let editType = log.type as unknown as EditType | undefined;
  if (!EDIT_TYPES.includes(editType!)) throw new Error(`unexpected edit ${editType}: ${describeNode(log)}`);
  let oldNode =
    log.oldNodePacked != null
      ? (makeDefaultObject(
          unpackBuiltinObject(unpackProtoJson(log.oldNodePacked), nodeType as unknown as ObjectType),
        ) as AnyNodeData)
      : null;
  let newNode =
    log.newNodePacked != null
      ? (makeDefaultObject(
          unpackBuiltinObject(unpackProtoJson(log.newNodePacked), nodeType as unknown as ObjectType),
        ) as AnyNodeData)
      : null;

  // invert if undo
  if (mode == "undo") {
    editType = UNDO_EDIT_BY_TYPE[editType!];
    if (editType == null) throw new Error(`cannot undo edit ${toCamelName(EditType, editType!)}: ${describeNode(log)}`);
    [oldNode, newNode] = [newNode, oldNode];
  }

  // shuffle oldNode/newNode :EditData
  if (
    editType == EditType.ARCHIVE ||
    editType == EditType.UNARCHIVE ||
    editType == EditType.DELETE ||
    editType == EditType.RESTORE
  ) {
    oldNode = oldNode ?? newNode; // oldNode is always set
    if (oldNode == null) throw new Error(`missing old node for ${editType}: ${describeNode(log)}`);

    newNode = null; 
  }

  // make edit
  const subjectPtr = options?.subjectPtr ?? userPtr.value;
  if (subjectPtr == null) throw new Error(`missing subject for ${log.type}: ${describeNode(log)}`);
  const scope = makeScope({ benchId: log.benchPtr?.id, packageId: log.packagePtr?.id });
  const edit: EditData = {
    metatype: ObjectType.EDIT,
    id: newEditId(),
    type: editType!,
    nodePtr: log.nodePtr,
    scope: scope,
    properties: log.properties,
    oldNodePartial: oldNode != null ? wrapSomeNode(oldNode) : undefined,
    newNodePartial: newNode != null ? wrapSomeNode(newNode) : undefined,
    origin: origin.value,
    category: options?.category ?? log.category,
    subjectPtr: subjectPtr,
    editedAt: Timestamp.now(),
    undoOfPtr: mode == "undo" ? toPlainNodeRef(log) : undefined,
  };
  return edit;
}
