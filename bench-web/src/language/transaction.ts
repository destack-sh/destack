import { getPropertyType } from "@/language/field";
import { PartialNode, type ReadNodeGraph, type WriteNodeGraph } from "@/language/graph";
import { makeNode, NodeIn } from "@/language/node";
import { packValue, unpackValue } from "@/language/value";
import { getCachedGraphClient, HUMANIZED_OPERATION_STATUS } from "@/proto/services";
import {
  BlockProperty,
  ChangeCategory,
  CommitTransactionRequest,
  EditOperationData,
  EditOperationType,
  EditType,
  GraphScopeData,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  NodeMode,
  TextData,
  Timestamp,
  type AnyNodeData,
  type EditData,
  type NodeTypeMapping,
} from "@/proto/wire";
import {
  arrayEquals,
  describeEdit,
  describeNode,
  EMPTY_SCOPE,
  makeScope,
  nodeReference,
  propertyInfo,
  toNodeRef,
  unwrapSomeNode,
  wrapSomeNode,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { nonce, origin, userOrNullPtr, userPtr } from "@/system/client";
import { toaster } from "@/ui/toast";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { toValueRef } from "@/utils/ref";
import { uuidt } from "@/utils/uuidt";
import type { RpcError } from "grpc-web";
import { computed, nextTick, ref, shallowRef, toValue, triggerRef, watch, type MaybeRef, type Ref } from "vue";

export type DebounceLevel = "tick" | "short" | "long";
const DEBOUNCE_LEVELS: Record<"short" | "long", number> = {
  short: 500,
  long: 2000,
};

const IMPLICIT_UPDATE_PROPERTIES_IDS = ["updatedAt", "updatedEpoch", "updatedByPtr", "deletedAt"].map(
  (p) => BlockProperty[p as any] as unknown as number,
);
const NONCE_POSTFIX = nonce.replace("-", "").slice(0, 16);

export function newChangeId(): string {
  return uuidt({ nonce: NONCE_POSTFIX });
}

export function newEditId(): string {
  return uuidt({ nonce: NONCE_POSTFIX });
}

export function newTransactionId(): string {
  return uuidt({ nonce: NONCE_POSTFIX });
}

//
// Transaction
//

/** Metadata for a transaction (mostly local only). */
export type TransactionMeta = {
  connectionId?: number | null;
  subject?: MaybeRef<NodeReferenceData | null>;
  change?: ChangeIn;
  category?: ChangeCategory;
};

/** A change for grouping edits together. */
export type ChangeIn = {
  key: string;
  title?: string;
  text?: TextData;
};

export type TransactionOptions = {
  debounce?: DebounceLevel;
};

/** A transaction on the Bench state graph. */
export type Transaction = TransactionMeta & {
  readonly scope: GraphScopeData;
  readonly id: string;
  readonly edits: EditData[];
  describeSelf(): string;

  /** Adds an externally created edit */
  addEdit(edit: EditData): void;
  /** Gets the sub tx for a specific connection */
  with(meta: TransactionMeta): Transaction;
  /** Stops debouncing the given edit (force start a new edit on that node) */
  clearDebounce(nodeId: string): void;
  /** Create a new node */
  create<T extends NodeType>(node: NodeIn<T>): NodeTypeMapping[T];
  /** Update regular properties in this node. v*/
  update<T extends AnyNodeData>(node: T, update: Partial<T> | EditOperationData[], options?: TransactionOptions): void;
  /** Move node between parents (and update it) */
  move<T extends AnyNodeData>(
    node: T,
    update: Partial<T> & { parentPtr: NodeReferenceData },
    options?: TransactionOptions,
  ): void;
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

  readonly _debouncedUpdatesByNodeId: Record<string, EditData> = {};
  readonly _subs: Array<(edit: EditData, meta: TransactionMeta, debounce: DebounceLevel | null) => void> = [];
  readonly _txByConnectionId: Record<number, TransactionBuilder> = {};

  constructor(id: string, scope: GraphScopeData) {
    this.id = id;
    this.scope = scope;
    this.benchPtr = scope.benchId != null ? nodeReference(NodeType.BENCH, scope.benchId) : null;
  }

  clearDebounce(nodeId: string) {
    if (!this._debouncedUpdatesByNodeId[nodeId]) return;
    delete this._debouncedUpdatesByNodeId[nodeId];
  }
}

/**
 * Build a transaction (maybe for a specific connection/change).
 * We often want to share transaction state across multiple builders with curried info (connectionId, change, category, ...).
 */
export class TransactionBuilder implements Transaction {
  public readonly connectionId: number | undefined;
  public readonly subject: MaybeRef<NodeReferenceData | null>;
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
    this.subject = tx.subject;
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

  clearDebounce(nodeId: string) {
    this.state.clearDebounce(nodeId);
  }

  with(meta: TransactionMeta): Transaction {
    if (
      meta.connectionId == this.connectionId &&
      meta.change?.key == this.change?.key &&
      meta.category == this.category
    ) {
      return this; // no change
    }

    let base: TransactionBuilder = this;

    // cache by connection id
    if (meta.connectionId != null) {
      if (this.state._txByConnectionId[meta.connectionId] == null) {
        this.state._txByConnectionId[meta.connectionId] = new TransactionBuilder({
          state: this.state,
          subject: this.subject,
          connectionId: meta.connectionId,
          change: meta.change ?? this.change,
          category: meta.category ?? this.category,
        });
      }
      base = this.state._txByConnectionId[meta.connectionId];
    }

    // and split if change/category is specified
    if (meta.change != null || meta.category != null) {
      return new TransactionBuilder({
        state: this.state,
        connectionId: meta.connectionId === null ? undefined : base.connectionId,
        subject: base.subject,
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
    editType: EditType.CREATE | EditType.UPSERT | EditType.DELETE | EditType.RESTORE | EditType.ERASE,
    node: AnyNodeData,
    debounce: DebounceLevel | null,
  ) {
    this.checkInScope(node);

    // make edit & notify
    const subjectPtr = toValue(this.subject);
    if (subjectPtr == null) throw new Error("no subject for edit");
    const edit: EditData = {
      metatype: ObjectType.EDIT,
      id: newEditId(),
      type: editType,
      nodePtr: toNodeRef(node),
      scope: this.getScope(node),
      origin: origin.value,
      subjectPtr: subjectPtr,
      changeKey: this.change?.key,
      category: this.category,
      editedAt: Timestamp.now(),
      oldEditedAt: node.deletedAt,
      nodeData: wrapSomeNode(node),
      operations: [],
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

    // inject :Tracing
    if ("mode" in properties && ((nodeIn as any).mode == null || (nodeIn as any).mode == 0)) {
      (nodeIn as any).mode = NodeMode.PRODUCTION; // should this be relative to the current Space.mode?
    }

    // create node
    // NOTE :Cleanup: why doesn't makeNode typecheck properly here?
    const node: NodeTypeMapping[T] =
      nodeIn.id == null ? makeNode(nodeIn as any) : (nodeIn as unknown as NodeTypeMapping[T]);

    this._addSimpleEdit(EditType.CREATE, node, null);
    return node as NodeTypeMapping[T];
  }

  _doUpdate<T extends AnyNodeData>(
    editType: EditType.UPDATE | EditType.MOVE,
    node: T,
    update: Partial<T> | EditOperationData[],
    options?: TransactionOptions,
  ) {
    this.checkInScope(node);

    // convert update to operations
    let operations: EditOperationData[];
    if (!Array.isArray(update)) {
      operations = makeEdit(node, update as NodeIn<any>);
    } else {
      operations = update;
    }

    if (!options?.debounce || !this.state._debouncedUpdatesByNodeId[node.id]) {
      // create new edit
      const subjectPtr = toValue(this.subject);
      if (subjectPtr == null) throw new Error("no subject for edit");
      const edit: EditData = {
        metatype: ObjectType.EDIT,
        id: newEditId(),
        type: editType,
        nodePtr: toNodeRef(node),
        scope: this.getScope(node),
        operations: operations,
        origin: origin.value,
        subjectPtr: subjectPtr,
        changeKey: this.change?.key,
        category: this.category,
        editedAt: Timestamp.now(),
      };
      if (options?.debounce) {
        this.state._debouncedUpdatesByNodeId[node.id] = edit;
      }
      this.state.edits.push(edit);
      this._notifyEdit(edit, options?.debounce ?? null);
    } else {
      // merge into existing edit & notify directly :DebouncedUpdate
      const edit = this.state._debouncedUpdatesByNodeId[node.id];
      for (const op of operations) {
        const existingOp = edit.operations.find((o) => arrayEquals(o.path, op.path));
        if (existingOp != null) {
          existingOp.type = op.type;
          existingOp.newValuePacked = op.newValuePacked;
        } else {
          edit.operations.push(op);
        }
      }
      // coalesce successive move/update into move edit
      if (editType == EditType.MOVE && edit.type != EditType.MOVE) {
        edit.type = EditType.MOVE;
      }
      this._notifyEdit(edit, options?.debounce);
    }
  }

  update<T extends AnyNodeData>(node: T, update: Partial<T> | EditOperationData[], options?: TransactionOptions) {
    this._doUpdate(EditType.UPDATE, node, update, options);
  }

  move<T extends AnyNodeData>(
    node: T,
    update: (Partial<T> & { parentPtr: NodeReferenceData }) | EditOperationData[],
    options?: TransactionOptions,
  ) {
    this._doUpdate(EditType.MOVE, node, update, options);
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

/** Turns a top-level node partial update into its corresponding edit operations (with subnode edits) */
export function makeEdit<T extends NodeType>(node: NodeTypeMapping[T], update: NodeIn<T>): EditOperationData[] {
  // root + subnode properties
  if ("subnode" in update) {
    const subnodeOperations = makeEditFromSubnode(node, update);
    // strip metatype from update, and 'type' if it hasn't changed
    // (they're just used for getting the right properties & type checking)
    delete (update as any).metatype;
    if ((node as any).type == update.type) delete (update as any).type;
    const rootOperations = makeEditFromRoot(node, update as any);
    return [...rootOperations, ...subnodeOperations];
  } else {
    // just root properties
    return makeEditFromRoot(node, update as any);
  }
}

/** Turns a top-level node partial update into its corresponding edit operations */
export function makeEditFromRoot<T extends AnyNodeData>(node: T, update: Partial<T>): EditOperationData[] {
  const operations: EditOperationData[] = [];
  const propertiesEnum = PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const properties = PROPERTY_INFOS_BY_TYPE[node.metatype]!;

  // regular properties
  for (const key in update) {
    const propId = propertiesEnum[key as unknown as number];
    if (propId == null) {
      if (key == "subnode") continue; // subnode is handled separately below
      throw new Error(`missing property ${key} in ${node.metatype}`);
    }
    const prop = properties[propId];
    const propType = getPropertyType(prop);
    const newValue = (update as any)[key];
    let operation: EditOperationData;
    const oldValuePacked = packValue((node as any)[key], propType, { wrapScalar: false });
    if (newValue == null) {
      operation = {
        metatype: ObjectType.EDIT_OPERATION,
        type: EditOperationType.CLEAR,
        path: [propId.toString()],
        oldValuePacked,
      };
    } else {
      const newValuePacked = packValue(newValue, propType, { wrapScalar: false });
      operation = {
        metatype: ObjectType.EDIT_OPERATION,
        type: EditOperationType.SET,
        path: [propId.toString()],
        newValuePacked,
        oldValuePacked,
      };
    }
    operations.push(operation);
  }
  return operations;
}

const NODE_SUBTYPE_PACKED_ID = BlockProperty.subnodePacked;
const NODE_SUBTYPE_PACKED_KEY = NODE_SUBTYPE_PACKED_ID.toString(); // it's the same property id for all nodes

/** Turns a top level subnode edit into corresponding edit operations (only for that subnode) */
export function makeEditFromSubnode<T extends NodeType>(
  node: NodeTypeMapping[T],
  update: NodeIn<T>,
): EditOperationData[] {
  if (!("subnode" in update)) {
    return [];
  }
  const operations: EditOperationData[] = [];
  const propertiesEnum = PROPERTY_ENUM_BY_SUBTYPE[update.metatype as NodeType]?.[update.type];
  const properties: Record<number, PropertyInfo> | undefined =
    PROPERTY_INFOS_BY_SUBTYPE[update.metatype as NodeType]?.[update.type];
  if (propertiesEnum == null || properties == null)
    throw new Error(`missing properties for ${NodeType[update.metatype]}.${update.type.toString()}`);
  const subtypeKey = update.type.toString();

  // ignore type/metatype in subnode update
  for (const key in update.subnode) {
    const propId = propertiesEnum[key as unknown as number];
    if (propId == null) {
      throw new Error(`missing property ${key} in ${node.metatype}`);
    }
    const prop = properties[propId];
    const propType = getPropertyType(prop);
    const newValue = (update.subnode as any)[key];
    let operation: EditOperationData;
    const oldValuePacked = packValue((node as any)[key], propType, { wrapScalar: false });
    if (newValue == null) {
      operation = {
        metatype: ObjectType.EDIT_OPERATION,
        type: EditOperationType.CLEAR,
        path: [NODE_SUBTYPE_PACKED_KEY, subtypeKey, propId.toString()],
        oldValuePacked,
      };
    } else {
      const newValuePacked = packValue(newValue, propType, { wrapScalar: false });
      operation = {
        metatype: ObjectType.EDIT_OPERATION,
        type: EditOperationType.SET,
        path: [NODE_SUBTYPE_PACKED_KEY, subtypeKey, propId.toString()],
        newValuePacked,
        oldValuePacked,
      };
    }
    operations.push(operation);
  }

  return operations;
}

/** Apply an edit operation to the given node */
export function applyEditOperation(operation: EditOperationData, node: AnyNodeData) {
  let obj: any = node;
  const rootProperty = propertyInfo(node.metatype, Number(operation.path[0]));
  for (let i = 0; i < operation.path.length; i++) {
    // map key
    let key = operation.path[i];
    const propId = Number(key);
    const isProperty = !Number.isNaN(propId) && (i == 0 || !rootProperty.isValuePacked);
    if (isProperty) {
      // builtin object property
      const objProperties = PROPERTY_ENUM_BY_TYPE[obj.metatype as ObjectType];
      const propName = objProperties?.[propId];
      if (propName == null) {
        break; // invalid path
      }
      key = propName;
    }

    if (i < operation.path.length - 1) {
      // next: descend into value
      let nextObj = obj[key];
      if (nextObj == null) {
        // create object
        nextObj = {};
        Object.assign(obj, { [key]: nextObj });
      }
      obj = nextObj;
    } else {
      // done: set value
      let newValue: any;
      if (isProperty) {
        // builtin object property
        const objProperties = PROPERTY_INFOS_BY_TYPE[obj.metatype as ObjectType];
        const prop = objProperties[propId];
        const valueType = getPropertyType(prop);
        newValue = unpackValue(operation.newValuePacked!, valueType, { wrapScalar: false });
      } else {
        // custom object field (packed by default)
        newValue = operation.newValuePacked;
      }
      Object.assign(obj, { [key]: newValue });
    }
  }
}

/**
 * Applies the edits to the graph (in place!).
 * If a base graph is given, this graph is assumed to be an overlay.
 */
export function editGraph(
  graph: ReadNodeGraph & WriteNodeGraph,
  edits: EditData[],
  options?: { base?: ReadNodeGraph; ignoreMissing?: boolean },
) {
  for (const edit of edits) {
    if (
      edit.type == EditType.CREATE ||
      edit.type == EditType.UPSERT ||
      (edit.type == EditType.RESTORE && !graph.has(edit.nodePtr!))
    ) {
      // add
      if (edit.nodeData == null) throw new Error(`missing node in edit: ${describeEdit(edit)}`);
      const node = unwrapSomeNode(edit.nodeData);
      // implicit metadata
      node.createdAt = node.updatedAt = edit.editedAt;
      node.deletedAt = undefined;
      if (edit.epoch != null && "createdEpoch" in node && "updatedEpoch" in node) {
        node.createdEpoch = node.updatedEpoch = edit.epoch;
      }
      node.createdByPtr = node.updatedByPtr = edit.subjectPtr;
      if (edit.type == EditType.CREATE || !graph.has(node)) {
        graph.add(node);
      } else {
        graph.update(node);
      }
    } else if (edit.type == EditType.ERASE && !(options?.base && !graph.has(edit.nodePtr!))) {
      // remove
      const oldNode = graph.get(edit.nodePtr!);
      if (!oldNode) {
        if (options?.ignoreMissing) {
          continue;
        }
        throw new Error(`missing node for delete: ${describeNode(edit.nodePtr!)} in ${graph.describeSelf()}`);
      }
      graph.remove(oldNode);
    } else {
      // update
      let updatedNode: AnyNodeData | null;
      if (edit.type == EditType.RESTORE) {
        if (edit.nodeData == null) {
          throw new Error(`missing node in edit: ${describeEdit(edit)} in ${graph.describeSelf()}`);
        }
        updatedNode = unwrapSomeNode(edit.nodeData);
      } else {
        updatedNode = graph.get(edit.nodePtr!);
        if (!updatedNode && options?.base) {
          updatedNode = options.base.get(edit.nodePtr!);
        }
        if (!updatedNode) {
          if (options?.ignoreMissing) {
            continue;
          }
          throw new Error(`missing node for update: ${describeNode(edit.nodePtr!)} in ${graph.describeSelf()}`);
        }
      }
      updatedNode = structuredClone(updatedNode); // copy

      // apply edit operations
      if (edit.type == EditType.UPDATE || edit.type == EditType.MOVE) {
        for (const operation of edit.operations) {
          applyEditOperation(operation, updatedNode);
        }
      }

      // implicit metadata
      updatedNode.updatedAt = edit.editedAt;
      if (edit.epoch != null && "updatedEpoch" in updatedNode) {
        updatedNode.updatedEpoch = edit.epoch;
      }
      updatedNode.updatedByPtr = edit.subjectPtr;
      if (edit.type == EditType.DELETE || edit.type == EditType.ERASE) {
        // (we handle removes here for overlays)
        updatedNode.deletedAt = edit.editedAt;
      } else if (edit.type == EditType.RESTORE) {
        updatedNode.deletedAt = undefined;
      }

      // extend setPaths for overlay
      if (options?.base != null) {
        if ((updatedNode as PartialNode<any>).setPaths == null) {
          (updatedNode as PartialNode<any>).setPaths = [...IMPLICIT_UPDATE_PROPERTIES_IDS.map((p) => [p.toString()])];
        }
        for (const op of edit.operations) {
          if (!(updatedNode as PartialNode<any>).setPaths.some((s: any) => arrayEquals(s, op.path))) {
            (updatedNode as PartialNode<any>).setPaths.push(op.path);
          }
        }
      }

      graph.update(updatedNode);
    }
  }
}

//
// Transaction buffers
//

type CommitFailure = {
  id: string;
  edits: EditData[];
  error: RpcError;
};
type BufferCallback = (
  event:
    | {
        type: "add";
        meta: TransactionMeta;
        newEdits: EditData[];
        connectionIdByEditId: Record<string, number>;
        debounce: DebounceLevel | null;
      }
    | {
        type: "reset";
        meta: TransactionMeta;
        newEdits: EditData[];
        connectionIdByEditId: Record<string, number>;
        oldEdits: EditData[];
      },
) => void;
type CommittedCallback = (event: {
  connectionIdByEditId: Record<string, number>;
  edits: EditData[];
  cascadedEdits: EditData[];
}) => void;
type AcceptedCallback = (event: { edits: EditData[]; cascadedEdits: EditData[] }) => void;

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
  /** Subscribes to *pending* edits from this buffer */
  subscribeBuffer(sub: BufferCallback): () => void;
  /** Accepts the given edits from an external source (does not trigger commit, just to mark them as successfully committed) */
  onCommitted(edits: EditData[], cascadedEdits: EditData[]): void;
  /** Force retries the given commit (for debugging) */
  retry?(id: string): Promise<void>;
  /** Gets the current buffered edits by their connection id (-1 if none) */
  getBufferByConnection(): Record<number, EditData[]>;

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
      // 'reset'
      newTx.state.edits.length = 0;
    });
    this._currentTx = newTx;
  }

  subscribeBuffer(sub: BufferCallback): () => void {
    // nothing to do
    return () => {};
  }

  getBufferByConnection(): Record<number, EditData[]> {
    return {};
  }

  onCommitted(edits: EditData[], cascadedEdits: EditData[]): void {
    // nothing to do
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
  private bufferSubs: Array<BufferCallback> = [];
  private currentTx: Transaction | null;
  private bufferedTx: Transaction | null;
  private bufferedEditsById: Record<string, EditData> = {};
  private bufferedConnectionByEditId: Record<string, number> = {};
  failedCommits: Ref<Record<string, CommitFailure>> = shallowRef({});

  constructor(id: number, scope: GraphScopeData) {
    this.id = id;
    this.scope = scope;
    this.currentTx = null;
    this.bufferedTx = null;
    this.reset();
  }

  get tx(): Transaction {
    if (this.currentTx == null) throw new Error("no active transaction");
    return this.currentTx;
  }

  async commit() {
    if (this.currentTx == null) throw new Error("no active transaction");
    if (this.bufferedTx != null) throw new Error(`transaction ${this.bufferedTx.id} is already committing`);
    try {
      log.trace("transaction.commit", { scope: this.scope, id: this.currentTx.id, edits: this.currentTx.edits });

      const client = getCachedGraphClient(this.scope);
      if (client == null) throw new Error(`no client for ${this.scope}`);

      // swap
      const edits = this.currentTx.edits;
      this.bufferedTx = this.currentTx;
      this.currentTx = this._makeCurrentTx();

      // commit
      const {
        response: { epoch, cascadedEdits },
      } = await client.commitTransaction(
        { edits, id: this.bufferedTx.id, scope: this.scope },
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
    } catch (error) {
      // failed
      const fail: CommitFailure = { id: this.bufferedTx!.id, edits: this.bufferedTx!.edits, error: error as RpcError };
      this.failedCommits.value[fail.id] = fail;
      triggerRef(this.failedCommits);

      // rollback
      log.error("transaction.commit.error", { scope: this.scope, error });
      toaster.error({
        title: HUMANIZED_OPERATION_STATUS[(error as RpcError).code] ?? "Synchronization error",
        text: `Synchronizing ${this.bufferedTx?.edits.length ?? 0} edits failed: ${IS_DEV ? (error as Error).message : (error as RpcError).code}`,
      });
      this.reset();
    } finally {
      this.bufferedTx = null;
    }
  }

  async reset() {
    this.currentTx = this._makeCurrentTx();
    const oldEdits = Object.values(this.bufferedEditsById);
    this.bufferedEditsById = {};
    this.bufferedConnectionByEditId = {};
    this.bufferedTx = null;
    this.bufferSubs.forEach((sub) =>
      sub({ type: "reset", meta: {}, connectionIdByEditId: {}, newEdits: [], oldEdits }),
    );
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
      if (this.currentTx?.id !== tx.id) {
        throw new Error(`transaction ${tx.describeSelf()} is closed`);
      }
      const pendingConnectionId = this.bufferedConnectionByEditId[edit.id];
      if (pendingConnectionId != null && (!meta.connectionId || meta.connectionId != pendingConnectionId)) {
        // ensure connections don't trample on each others' edits since we currently only optimistically overlay
        //  edits from the same connection (see connection)
        throw new Error(`edit ${edit.id} already pending in ${pendingConnectionId}`);
      }
      this.bufferedEditsById[edit.id] = edit;
      if (meta.connectionId != null) {
        this.bufferedConnectionByEditId[edit.id] = meta.connectionId;
      }
      this.bufferSubs.forEach((sub) =>
        sub({
          type: "add",
          meta,
          newEdits: [edit],
          connectionIdByEditId: { [edit.id]: this.bufferedConnectionByEditId[edit.id] },
          debounce,
        }),
      );
    });
    return tx;
  }

  getBufferByConnection(): Record<number, EditData[]> {
    const bufferedEditsByConnection: Record<number, EditData[]> = {};
    for (const edit of Object.values(this.bufferedEditsById)) {
      const connectionId = this.bufferedConnectionByEditId[edit.id] ?? -1;
      if (!bufferedEditsByConnection[connectionId]) bufferedEditsByConnection[connectionId] = [];
      bufferedEditsByConnection[connectionId].push(edit);
    }
    return bufferedEditsByConnection;
  }

  subscribeBuffer(sub: BufferCallback): () => void {
    this.bufferSubs.push(sub);
    return () => {
      const idx = this.bufferSubs.indexOf(sub);
      if (idx >= 0) this.bufferSubs.splice(idx, 1);
    };
  }

  onCommitted(edits: EditData[], cascadedEdits: EditData[]): void {
    // update buffer subscribers
    const oldEdits = Object.values(this.bufferedEditsById);
    let bufferChanged = false;
    for (const edit of edits) {
      if (this.bufferedEditsById[edit.id]) {
        delete this.bufferedEditsById[edit.id];
        if (this.bufferedConnectionByEditId[edit.id] != null) delete this.bufferedConnectionByEditId[edit.id];
        bufferChanged = true;
      }
    }
    if (bufferChanged) {
      const newBufferedEdits = Object.values(this.bufferedEditsById);
      const newBufferedEditsByConnection: Record<number, EditData[]> = {};
      for (const edit of newBufferedEdits) {
        const connectionId = this.bufferedConnectionByEditId[edit.id] ?? -1;
        if (!newBufferedEditsByConnection[connectionId]) newBufferedEditsByConnection[connectionId] = [];
        newBufferedEditsByConnection[connectionId].push(edit);
      }
      this.bufferSubs.forEach((sub) => {
        sub({
          type: "reset",
          meta: {},
          newEdits: newBufferedEditsByConnection[-1] ?? [],
          connectionIdByEditId: this.bufferedConnectionByEditId,
          oldEdits,
        });
        for (const connectionId of Object.keys(newBufferedEditsByConnection)) {
          sub({
            type: "add",
            meta: { connectionId: parseInt(connectionId) },
            newEdits: newBufferedEditsByConnection[parseInt(connectionId)]!,
            connectionIdByEditId: this.bufferedConnectionByEditId,
            debounce: null,
          });
        }
      });
    }
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
    return this.bufferedTx != null;
  }
}

//
// Transaction buffer management
//

let bufferId = 0;
export function newBufferId() {
  return bufferId++;
}
const globalTxBuffer: TransactionBuffer = new RemoteTransactionBuffer(newBufferId(), EMPTY_SCOPE);
const txBuffersByBenchId: Ref<Record<string, RemoteTransactionBuffer>> = shallowRef({});
export const txBuffers = computed(() => getAllTransactionBuffers());

export function getAllTransactionBuffers(): TransactionBuffer[] {
  return [globalTxBuffer, ...Object.values(txBuffersByBenchId.value)];
}

/**
 * Gets the transaction buffer for the given scope (non-exclusively).
 * We maintain one transaction buffer per Bench and one for other universal nodes (outside of Benches).
 * */
export function getTransactionBuffer(scope: GraphScopeData): TransactionBuffer {
  if (scope.benchId) {
    if (!txBuffersByBenchId.value[scope.benchId]) {
      const buffer = new RemoteTransactionBuffer(newBufferId(), scope);
      txBuffersByBenchId.value[scope.benchId] = buffer;
      triggerRef(txBuffersByBenchId);
      watchTransactionBuffer(buffer);
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
  let scheduledDebouncedCommitLevel: DebounceLevel | null = null;

  // watch pending edit
  buffer.subscribeBuffer((event) => {
    if (event.type != "add") return; // only react to added edits
    if (scheduledCommit) return; // already scheduled/committing
    if (event.debounce != null && event.debounce != "tick") {
      // schedule commit after debounce
      if (scheduledDebouncedCommit != null) clearTimeout(scheduledDebouncedCommit);
      scheduledDebouncedCommit = setTimeout(scheduleCommit, DEBOUNCE_LEVELS[event.debounce]);
      scheduledDebouncedCommitLevel = event.debounce;
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
      scheduledDebouncedCommitLevel = null;
      if (!buffer.isCommitting && !buffer.isPaused.value) {
        // commit if there is something to commit
        if (buffer.isDirty) buffer.commit();
      } else {
        // can't commit right now, try again in a bit
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
async function commitTransactionBuffers() {
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
export function startTransactionRotation() {
  if (_setupTransactionManagement) return;
  _setupTransactionManagement = true;
  // watch buffers
  watchTransactionBuffer(globalTxBuffer);
  // (the rest is watched on demand)
  // commit on user change
  watch(toValueRef(userPtr), () => commitTransactionBuffers());
  // commit before exit
  window.addEventListener("beforeunload", (e) => commitTransactionBuffers());
}

//
// Edit handling
//

export const EDIT_TYPES = [
  EditType.CREATE,
  EditType.UPSERT,
  EditType.UPDATE,
  EditType.MOVE,
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
  [EditType.DELETE]: EditType.RESTORE,
  [EditType.RESTORE]: EditType.DELETE,
};
