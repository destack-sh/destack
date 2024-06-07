import { HUMANIZED_OPERATION_STATUS, getHostClient, supervisor } from "@/proto/services";
import {
  BlockProperty,
  CommitTransactionRequest,
  EditType,
  GraphScope,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  Struct as ProtoStruct,
  Timestamp,
  type AnyNodeData,
  type EditData,
  type IGraphIOClient,
  type NodeTypeMapping,
  type PropertyInfo,
} from "@/proto/wire";
import {
  describeEdit,
  describeNode,
  isNode,
  makeNode,
  nodeReference,
  toNodeReference,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { nonce, origin, userOrNullPtr, userPtr } from "@/system/client";
import { type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { makeIcon } from "@/system/icon";
import { toaster } from "@/system/toast";
import {
  getTypeIdentityForProperty,
  packBuiltinObject,
  packValue,
  unpackBuiltinObject,
  type JsonValue,
} from "@/system/value";
import { AsyncEvent } from "@/utils/functools";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { toValueRef } from "@/utils/ref";
import { uuidt } from "@/utils/uuidt";
import type { RpcError } from "grpc-web";
import { nextTick, ref, shallowRef, toRef, triggerRef, watch, type MaybeRef, type Ref } from "vue";

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
  readonly scope: GraphScope;
  readonly id: string;
  readonly edits: EditData[];
  describeSelf(): string;

  /** Adds an externally created edit */
  addEdit(edit: EditData): void;

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

export class TransactionBuilder implements Transaction {
  public readonly scope: GraphScope;
  public readonly id: string;
  public readonly subjectRef: Ref<NodeReferenceData | null>;
  public readonly edits: EditData[] = [];
  private readonly debouncedUpdates: Record<string, EditData> = {};
  private subs: Array<(edit: EditData, debounce: DebounceLevel | null) => void> = [];
  private benchPtr: TypedNodeReferenceData<NodeType.BENCH> | null;

  constructor(scope: GraphScope, id: string, subject: MaybeRef<NodeReferenceData | null>) {
    this.scope = scope;
    this.id = id;
    this.subjectRef = toRef(subject);
    this.benchPtr = scope.benchId != null ? nodeReference(NodeType.BENCH, scope.benchId) : null;
  }

  get subject(): NodeReferenceData {
    if (!this.subjectRef.value) throw new Error("subject not set");
    return this.subjectRef.value;
  }

  describeSelf(): string {
    return `Transaction(${this.id}, ${this.edits.length} edits)`;
  }

  onEdit(sub: (edit: EditData, debounce: DebounceLevel | null) => void): () => void {
    this.subs.push(sub);
    return () => {
      const idx = this.subs.indexOf(sub);
      if (idx >= 0) this.subs.splice(idx, 1);
    };
  }

  _getScope(node: AnyNodeData): GraphScope {
    const allProperties = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
    const benchId = (node as any).benchPtr?.id ?? this.scope.benchId;
    const packageId = (node as any).packagePtr?.id ?? this.scope.packageId;
    if ("packagePtr" in allProperties && packageId == null)
      throw new Error(`missing packagePtr in ${describeNode(node)}`);
    return { benchId, packageId, transactionId: this.id };
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
    // pack 'old' and 'new' node delta
    let newNodePacked = undefined;
    let oldNodePacked = undefined;
    if (editType == EditType.CREATE || editType == EditType.UPSERT) {
      newNodePacked = packNodeDelta(node);
    } else if (editType == EditType.ERASE || editType == EditType.ARCHIVE || editType == EditType.DELETE) {
      if (node.deletedAt != null || node.archivedAt != null) {
        node = { ...node, deletedAt: undefined, archivedAt: undefined };
      }
      oldNodePacked = packNodeDelta(node);
    } else if (editType == EditType.UNARCHIVE) {
      // remember old 'archived_at' in old node, put full restored node in new node
      oldNodePacked = packNodeDelta(node, { only: ["archivedAt"] });
      newNodePacked = packNodeDelta({ ...node, archivedAt: undefined });
    } else if (editType == EditType.RESTORE) {
      // remember old 'deleted_at' in old node, put full restored node in new node
      oldNodePacked = packNodeDelta(node, { only: ["deletedAt"] });
      newNodePacked = packNodeDelta({ ...node, deletedAt: undefined });
    }

    // make edit & notify
    const edit: EditData = {
      id: newEditId(),
      type: editType,
      nodePtr: toNodeReference(node),
      scope: this._getScope(node),
      oldNodePacked,
      newNodePacked,
      properties: [],
      origin: origin.value,
      subjectPtr: this.subject,
      editedAt: Timestamp.now(),
    };
    this.edits.push(edit);
    this._notifyEdit(edit, debounce);
    return edit;
  }

  _notifyEdit(edit: EditData, debounce: DebounceLevel | null) {
    for (const sub of this.subs) {
      sub(edit, debounce);
    }
  }

  addEdit(edit: EditData): void {
    this.edits.push(edit);
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
        (nodeIn as any).benchPtr = this.benchPtr; // infer bench
      }
      if ((nodeIn as any).benchPtr?.id != this.benchPtr?.id) {
        throw new Error(
          `node from other benchPtr: ${describeNode(nodeIn)} != ${this.benchPtr != null ? describeNode(this.benchPtr) : "<null>"}`,
        );
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

    if (!options?.debounce || !this.debouncedUpdates[node.id]) {
      // create new edit
      const oldNodePacked: Record<string, JsonValue> = {};
      const newNodePacked: Record<string, JsonValue> = {};
      for (const prop of properties) {
        const typeInfo = getTypeIdentityForProperty(prop);
        const propName = propertiesEnum[prop.id];
        const oldValue = (node as any)[propName];
        const newValue = (update as any)[propName];
        const { valuePacked: oldValuePacked } = packValue(oldValue, typeInfo, null, { wrapPrimitive: false });
        const { valuePacked: newValuePacked } = packValue(newValue, typeInfo, null, { wrapPrimitive: false });
        oldNodePacked[prop.id.toString()] = oldValuePacked;
        newNodePacked[prop.id.toString()] = newValuePacked;
      }
      const edit: EditData = {
        id: newEditId(),
        type: editType,
        nodePtr: toNodeReference(node),
        scope: this._getScope(node),
        properties: properties.map((p) => p.id),
        oldNodePacked: ProtoStruct.fromJson(oldNodePacked),
        newNodePacked: ProtoStruct.fromJson(newNodePacked),
        origin: origin.value,
        subjectPtr: this.subject,
        editedAt: Timestamp.now(),
      };
      if (options?.debounce) {
        this.debouncedUpdates[node.id] = edit;
      }
      this.edits.push(edit);
      this._notifyEdit(edit, options?.debounce ?? null);
    } else {
      // merge into existing edit & notify directly :DebouncedUpdate
      const edit = this.debouncedUpdates[node.id];
      if (edit.oldNodePacked == null || edit.newNodePacked == null) {
        throw new Error(`missing old/new node in debounced edit: ${describeEdit(edit)}`);
      }
      const oldNodePacked = ProtoStruct.toJson(edit.oldNodePacked) as Record<string, JsonValue>;
      const newNodePacked = ProtoStruct.toJson(edit.newNodePacked) as Record<string, JsonValue>;
      for (const prop of properties) {
        const propName = propertiesEnum[prop.id];
        const typeInfo = getTypeIdentityForProperty(prop);
        // add to Edit.properties if not there yet
        if (!edit.properties.includes(prop.id)) {
          edit.properties.push(prop.id);
        }
        // add old value if it doesn't already exist
        if (oldNodePacked[prop.id.toString()] == null) {
          const oldValue = (node as any)[propName];
          const { valuePacked: oldValuePacked } = packValue(oldValue, typeInfo, null, { wrapPrimitive: false });
          oldNodePacked[prop.id.toString()] = oldValuePacked;
        }
        // and update new value
        const newValue = (update as any)[propName];
        const { valuePacked: newValuePacked } = packValue(newValue, typeInfo, null, { wrapPrimitive: false });
        newNodePacked[prop.id.toString()] = newValuePacked;
      }
      edit.oldNodePacked = ProtoStruct.fromJson(oldNodePacked);
      edit.newNodePacked = ProtoStruct.fromJson(newNodePacked);
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

export function packNodeDelta(node: AnyNodeData, options?: { only?: string[] }): ProtoStruct {
  const nodePacked = packBuiltinObject(node, options);
  return ProtoStruct.fromJson(nodePacked);
}

export function unpackNodeDelta(nodePackedStruct: ProtoStruct, nodeType?: NodeType): AnyNodeData {
  const nodePacked = ProtoStruct.toJson(nodePackedStruct);
  const node = unpackBuiltinObject(nodePacked, nodeType as unknown as ObjectType);
  if (!isNode(node)) throw new Error(`unexpected node data: ${node.metatype}`);
  return node;
}

/**
 * Applies the edits to the graph (in place!).
 */
export function editGraph(
  graph: ReadNodeGraph & WriteNodeGraph,
  edits: EditData[],
  options?: { isOverlayOf?: ReadNodeGraph },
) {
  for (const edit of edits) {
    const nodeType = edit.nodePtr!.type;
    if (edit.type == EditType.CREATE || edit.type == EditType.UPSERT) {
      // add
      if (edit.newNodePacked == null) throw new Error(`missing newNodePacked in edit: ${describeEdit(edit)}`);
      const newNodeData = unpackNodeDelta(edit.newNodePacked);
      // implicit metadata
      newNodeData.createdAt = newNodeData.updatedAt = edit.editedAt;
      if (edit.epoch != null && "createdEpoch" in newNodeData && "updatedEpoch" in newNodeData) {
        newNodeData.createdEpoch = newNodeData.updatedEpoch = edit.epoch;
      }
      newNodeData.createdByPtr = newNodeData.updatedByPtr = edit.subjectPtr;
      if (edit.type == EditType.CREATE || !graph.has(newNodeData)) {
        graph.add(newNodeData);
      } else {
        graph.update(newNodeData);
      }
    } else if (edit.type == EditType.ERASE && !(options?.isOverlayOf && !graph.has(edit.nodePtr!))) {
      // remove
      const oldNode = graph.get(edit.nodePtr!);
      if (!oldNode) throw new Error(`missing node for delete: ${edit.nodePtr!.id}`);
      graph.remove(oldNode);
    } else {
      // update
      let updatedNode: AnyNodeData | null;
      if (edit.type == EditType.UNARCHIVE || edit.type == EditType.RESTORE) {
        if (edit.oldNodePacked == null) throw new Error(`missing old node in edit: ${describeEdit(edit)}`);
        updatedNode = unpackNodeDelta(edit.oldNodePacked, nodeType);
      } else {
        updatedNode = graph.get(edit.nodePtr!);
        if (!updatedNode && options?.isOverlayOf) {
          updatedNode = options.isOverlayOf.get(edit.nodePtr!);
        }
        if (!updatedNode) {
          throw new Error(`missing node for update: ${edit.nodePtr!.id}`);
        }
      }
      updatedNode = { ...updatedNode }; // copy

      // directly edited properties
      if (edit.type == EditType.UPDATE || edit.type == EditType.MOVE) {
        if (edit.newNodePacked == null) throw new Error(`missing new node in edit: ${describeEdit(edit)}`);
        const newNode = unpackNodeDelta(edit.newNodePacked, nodeType);
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
      if (options?.isOverlayOf != null) {
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
  event: { type: "add"; edits: EditData[]; debounce: DebounceLevel | null } | { type: "reset"; edits: EditData[] },
) => void;
type CommittedCallback = (edits: EditData[]) => void;

/**
 * A transaction buffer provides Transactions and applies them to the graph.
 */
export interface TransactionBuffer {
  readonly id: number;
  /** Current active Transaction. */
  readonly tx: Transaction;
  /** Unconfirmed edits in active or pending transactions (for overlays). */
  readonly pendingEdits: EditData[];
  /** Retryable commits in case of error (for debugging).  */
  readonly failedCommits?: Readonly<Ref<Record<string, CommitFailure>>>;

  /** Commits the current transaction. */
  commit(): void | Promise<void>;
  /** Resets the current transaction and overlay. */
  reset(): void | Promise<void>;
  /** Accepts the given edits from an external source (does not trigger onCommitted) */
  accept(edits: EditData[]): void;
  /** Subscribes to *pending* edits from this buffer */
  subscribePending(sub: PendingCallback): () => void;
  /** Subscribes to *committed* edits from this buffer */
  subscribeCommitted(sub: CommittedCallback): () => void;
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
  public readonly scope: GraphScope;
  public readonly graph: ReadNodeGraph & WriteNodeGraph;
  public readonly pendingEdits = [];
  public readonly isPaused: Ref<boolean> = ref(false);
  private acceptedSubs: Array<CommittedCallback> = [];
  private currentTx: TransactionBuilder | null = null; // always keep a single transaction

  constructor(id: number, scope: GraphScope, graph: ReadNodeGraph & WriteNodeGraph) {
    this.id = id;
    this.scope = scope;
    this.graph = graph;
    this.reset();
  }

  get tx(): Transaction {
    return this.currentTx!; // set in constructor
  }

  commit() {
    // nothing to do
  }

  reset() {
    // NOTE: we default to null user pointer in immediate transaction buffer since it's only used locally
    //  and we need some 'subject' to create edits (even when not connected to a 'real' remote graph)
    const newTx = new TransactionBuilder(this.scope, uuidt({ nonce: NONCE_POSTFIX }), userOrNullPtr);
    // immediately apply and reset the transaction
    newTx.onEdit((edit) => {
      if (this.currentTx !== newTx) throw new Error("transaction is closed");
      // apply edit directly
      editGraph(this.graph, [edit]);
      // notify
      this.acceptedSubs.forEach((sub) => sub([edit]));
      // 'reset'
      newTx.edits.length = 0;
    });
    this.currentTx = newTx;
  }

  accept(edits: EditData[]) {
    // nothing to do
  }

  subscribePending(sub: PendingCallback): () => void {
    // nothing to do
    return () => {};
  }

  subscribeCommitted(sub: CommittedCallback): () => void {
    this.acceptedSubs.push(sub);
    return () => {
      const idx = this.acceptedSubs.indexOf(sub);
      if (idx >= 0) this.acceptedSubs.splice(idx, 1);
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
  public readonly scope: GraphScope;
  public readonly client: IGraphIOClient;
  public readonly isPaused: Ref<boolean> = ref(false);
  private pendingSubs: Array<PendingCallback> = [];
  private committedSubs: Array<CommittedCallback> = [];
  private currentTx: Transaction | null;
  private pendingTx: Transaction | null;
  private pendingEditsById: Record<string, EditData> = {};
  failedCommits: Ref<Record<string, CommitFailure>> = shallowRef({});

  constructor(id: number, scope: GraphScope, client: IGraphIOClient) {
    this.id = id;
    this.scope = scope;
    this.currentTx = null;
    this.pendingTx = null;
    this.client = client;
    this.reset();
  }

  get tx(): Transaction {
    if (this.currentTx == null) throw new Error("no active transaction");
    return this.currentTx;
  }

  get pendingEdits() {
    return Object.values(this.pendingEditsById);
  }

  async commit() {
    if (this.currentTx == null) throw new Error("no active transaction");
    if (this.pendingTx != null) throw new Error(`transaction ${this.pendingTx.describeSelf()} is already committing`);
    try {
      log.trace("transaction.commit", { scope: this.scope, id: this.currentTx.id, edits: this.currentTx.edits });

      // swap
      const edits = this.currentTx.edits;
      this.pendingTx = this.currentTx;
      this.currentTx = this._makeCurrentTx();

      // commit
      const {
        response: { epoch, revisions },
      } = await this.client.commitTransaction(
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
    this.pendingTx = null;
    this.pendingSubs.forEach((sub) => sub({ type: "reset", edits: [] }));
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
    const tx = new TransactionBuilder(this.scope, newTransactionId(), userPtr);
    tx.onEdit((edit, debounce) => {
      if (this.currentTx !== tx) throw new Error(`transaction ${tx.describeSelf()} is closed`);
      this.pendingEditsById[edit.id] = edit;
      this.pendingSubs.forEach((sub) => sub({ type: "add", edits: [edit], debounce }));
    });
    return tx;
  }

  accept(edits: EditData[]): void {
    let pendingEditsChanged = false;
    for (const edit of edits) {
      if (this.pendingEditsById[edit.id]) {
        delete this.pendingEditsById[edit.id];
        pendingEditsChanged = true;
      }
    }
    if (pendingEditsChanged) {
      const newPendingEdits = Object.values(this.pendingEditsById);
      this.pendingSubs.forEach((sub) => sub({ type: "reset", edits: newPendingEdits }));
    }
  }

  subscribePending(sub: PendingCallback): () => void {
    this.pendingSubs.push(sub);
    return () => {
      const idx = this.pendingSubs.indexOf(sub);
      if (idx >= 0) this.pendingSubs.splice(idx, 1);
    };
  }

  subscribeCommitted(sub: CommittedCallback): () => void {
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
      key: `transaction.togglePaused.${this.id}`,
      title: this.isPaused.value ? "Buffer paused" : "Buffer resumed",
      text: `Buffer ${this.id} is ${this.isPaused.value ? "pausing transactions" : "resuming transactions"}.`,
      override: true,
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
const globalTxBuffer: TransactionBuffer = new RemoteTransactionBuffer(newBufferId(), {}, supervisor);
const txBuffersByBenchId: Ref<Record<string, RemoteTransactionBuffer>> = shallowRef({});
const txBufferLockByBenchId: Record<string, AsyncEvent> = {};

export function getAllTransactionBuffers(): TransactionBuffer[] {
  return [globalTxBuffer, ...Object.values(txBuffersByBenchId.value)];
}

/**
 * Gets the transaction buffer for the given scope (non-exclusively).
 * Currently we maintain one shared buffer per Bench and one for other global nodes.
 * */
export async function getTransactionBuffer(scope: GraphScope): Promise<TransactionBuffer> {
  if (scope.benchId) {
    if (!txBuffersByBenchId.value[scope.benchId]) {
      // synchronize so that only one buffer is created per bench even when called concurrently
      if (!txBufferLockByBenchId[scope.benchId]) {
        txBufferLockByBenchId[scope.benchId] = new AsyncEvent();
      } else {
        await txBufferLockByBenchId[scope.benchId].wait();
      }
      if (!txBuffersByBenchId.value[scope.benchId]) {
        const client = await getHostClient({ id: scope.benchId });
        const buffer = new RemoteTransactionBuffer(newBufferId(), scope, client);
        txBuffersByBenchId.value[scope.benchId] = buffer;
        triggerRef(txBuffersByBenchId);
        txBufferLockByBenchId[scope.benchId].set();
        delete txBufferLockByBenchId[scope.benchId];
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
  buffer.subscribePending((event) => {
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
