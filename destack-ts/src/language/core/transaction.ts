import { supergraph } from "@/globals";
import { DESTACK_ID, SYSTEM_ID } from "@/language/core/builtin";
import { isPageNode } from "@/language/core/const";
import { PartialNode, type ReadNodeGraph, type WriteNodeGraph } from "@/language/core/graph";
import { makeNode, NodeIn } from "@/language/core/node";
import { getPropertyType, TypeIdentity } from "@/language/core/type";
import { packValue, unpackValue } from "@/language/core/value";
import { unwrapBlockDefinition } from "@/language/source/block";
import { getCachedGraphClient, HUMANIZED_OPERATION_STATUS } from "@/proto/services";
import {
  DestackType,
  BlockData,
  BlockProperty,
  ChangeCategory,
  EditOperationData,
  EditOperationType,
  EditType,
  GraphScopeData,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeMode,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
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
  isNode,
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
import { IS_DEVELOPER_MODE } from "@/utils/globals";
import { log } from "@/utils/log";
import { toValueRef } from "@/utils/ref";
import { uuidt } from "@/utils/uuidt";
import { RpcError, StatusCode } from "grpc-web";
import { computed, nextTick, ref, shallowRef, toValue, triggerRef, watch, type MaybeRef, type Ref } from "vue";

export type DebounceLevel = "tick" | "short" | "long";
const DEBOUNCE_LEVELS: Record<"short" | "long", number> = {
  short: 500,
  long: 2000,
};

const IMPLICIT_UPDATE_PROPERTIES_IDS = ["updatedAt", "updatedByPtr", "archivedAt", "deletedAt"].map(
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

/** A transaction on the Destack state graph. */
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
  stopDebounce(nodeId: string): void;
  /** Make a Node (but not create it) for this Transaction */
  make<T extends NodeType>(nodeIn: NodeIn<T>): NodeTypeMapping[T];

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
  /** Archive node (incl. descendants) */
  archive(node: AnyNodeData): void;
  /** Unarchive node (incl. descendants) */
  unarchive(node: AnyNodeData): void;
  /** Soft delete node (incl. descendants), marked for later deletion after retention period */
  delete(node: AnyNodeData): void;
  /** Restore node from soft delete */
  restore(node: AnyNodeData): void;
};

/* Shared state to create multiple TransactionBuilder handles from different connections with same data */
export class TransactionState {
  readonly id: string;
  readonly scope: GraphScopeData;
  readonly destackPtr: TypedNodeReferenceData<NodeType.SPACE> | null;
  readonly edits: EditData[] = [];

  readonly _debouncedUpdatesByNodeId: Record<string, EditData> = {};
  readonly _subs: Array<(edit: EditData, meta: TransactionMeta, debounce: DebounceLevel | null) => void> = [];
  readonly _txByConnectionId: Record<number, TransactionBuilder> = {};

  constructor(id: string, scope: GraphScopeData) {
    this.id = id;
    this.scope = scope;
    this.destackPtr = scope.destackId != null ? nodeReference(NodeType.SPACE, scope.destackId) : null;
  }

  stopDebounce(nodeId: string) {
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

  stopDebounce(nodeId: string) {
    this.state.stopDebounce(nodeId);
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
    const destackId = (node as any).destackPtr?.id ?? this.scope.destackId;
    const packageId = (node as any).packagePtr?.id;
    if ("packagePtr" in allProperties && packageId == null)
      throw new Error(`missing packagePtr in ${describeNode(node)}`);
    return makeScope({ destackId, packageIds: packageId != null ? [packageId] : this.scope.packageIds });
  }

  checkEdit(node: AnyNodeData) {
    if ("destackPtr" in node && (node.destackPtr?.id == DESTACK_ID || node.destackPtr?.id == SYSTEM_ID)) {
      throw new Error(`cannot edit in builtin destack: ${describeNode(node)}`);
    }
    if (this.scope.destackId != null && (!("destackPtr" in node) || node.destackPtr?.id != this.scope.destackId)) {
      throw new Error(`node from other destack: ${describeNode(node)} != ${this.scope.destackId}`);
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
    this.checkEdit(node);

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
      oldEditedAt: editType == EditType.ARCHIVE ? node.archivedAt : node.deletedAt,
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

  make<T extends NodeType>(nodeIn: NodeIn<T>): NodeTypeMapping[T] {
    // fill in scope
    const properties = PROPERTY_ENUM_BY_TYPE[nodeIn.metatype as unknown as ObjectType];
    if (properties == null) {
      throw new Error(`missing properties for ${nodeIn.metatype}`);
    } else if ("packagePtr" in properties && (nodeIn as any).packagePtr == null) {
      throw new Error(`missing packagePtr in ${describeNode(nodeIn)}`); // can't infer package
    } else if ("destackPtr" in properties) {
      if ((nodeIn as any).destackPtr == null) {
        if (this.state.destackPtr == null) {
          throw new Error(`missing destackPtr to infer destack for ${describeNode(nodeIn)}`);
        }
        (nodeIn as any).destackPtr = this.state.destackPtr; // infer destack
      }
    }

    // inject :Tracing
    if ("mode" in properties && ((nodeIn as any).mode == null || (nodeIn as any).mode == 0)) {
      (nodeIn as any).mode = NodeMode.MAIN; // should this be relative to the current Space.mode?
    }

    // create node
    const node: NodeTypeMapping[T] =
      nodeIn.id == null ? makeNode(nodeIn as any) : (nodeIn as unknown as NodeTypeMapping[T]);

    return node;
  }

  create<T extends NodeType>(nodeIn: NodeIn<T>): NodeTypeMapping[T] {
    const node = this.make(nodeIn);
    this._addSimpleEdit(EditType.CREATE, node, null);
    return node as NodeTypeMapping[T];
  }

  _doUpdate<T extends AnyNodeData>(
    editType: EditType.UPDATE | EditType.MOVE,
    node: T,
    update: Partial<T> | EditOperationData[],
    options?: TransactionOptions,
  ) {
    this.checkEdit(node);

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
    update: Partial<T> & { parentPtr: NodeReferenceData },
    options?: TransactionOptions,
  ) {
    // NOTE :Robustness: like in backend we should update computed ancestors on move :BadMoveAncestors
    this._doUpdate(EditType.MOVE, node, update, options);
  }

  _doRemove(editType: EditType.ARCHIVE | EditType.DELETE | EditType.ERASE, node: AnyNodeData, title: string) {
    let tx: TransactionBuilder = this;
    if (this.change?.key == null) {
      tx = tx.with({ change: { key: newChangeId(), title } }) as TransactionBuilder;
    }
    tx._addSimpleEdit(editType, { ...node }, null);

    // handle blocks and definitions together
    if (isNode(node, NodeType.BLOCK)) {
      // for definition blocks, also handle the source node
      const source = unwrapBlockDefinition(node);
      if (source?.definitionPtr?.id == node.id) {
        tx._addSimpleEdit(editType, source, null);
      }
    } else if (isPageNode(node) && node.definitionPtr != null) {
      // for inline source nodes, also handle the block definition
      const block = supergraph.getOrError(node.definitionPtr) as BlockData;
      tx._addSimpleEdit(editType, block, null);
    }
  }

  _doRecover(editType: EditType.RESTORE | EditType.UNARCHIVE, node: AnyNodeData, title: string) {
    let tx: TransactionBuilder = this;
    if (this.change?.key == null) {
      tx = tx.with({ change: { key: newChangeId(), title } }) as TransactionBuilder;
    }

    const cleanNode = { ...node };
    if (editType === EditType.UNARCHIVE) {
      cleanNode.archivedAt = undefined;
    } else if (editType === EditType.RESTORE) {
      cleanNode.deletedAt = undefined;
    }

    tx._addSimpleEdit(editType, cleanNode, null);

    // handle blocks and definitions together
    if (isNode(node, NodeType.BLOCK)) {
      // for definition blocks, also handle the source node
      const source = unwrapBlockDefinition(node);
      if (source?.definitionPtr?.id == node.id) {
        tx._addSimpleEdit(editType, source, null);
      }
    } else if (isPageNode(node) && node.definitionPtr != null) {
      // for inline source nodes, also handle the block definition
      const block = supergraph.getOrError(node.definitionPtr) as BlockData;
      tx._addSimpleEdit(editType, block, null);
    }
  }

  delete(node: AnyNodeData) {
    this._doRemove(EditType.DELETE, node, "Delete");
  }

  restore(node: AnyNodeData) {
    this._doRecover(EditType.RESTORE, node, "Restore");
  }

  archive(node: AnyNodeData) {
    this._doRemove(EditType.ARCHIVE, node, "Archive");
  }

  unarchive(node: AnyNodeData) {
    this._doRecover(EditType.UNARCHIVE, node, "Unarchive");
  }
}

/** Turns a top-level node partial update into its corresponding edit operations. */
export function makeEdit<T extends NodeType>(node: NodeTypeMapping[T], update: NodeIn<T>): EditOperationData[] {
  const operations: EditOperationData[] = [];
  const propertiesEnum = PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const properties = PROPERTY_INFOS_BY_TYPE[node.metatype]!;

  // regular properties
  for (const key in update) {
    const propId = propertiesEnum[key as unknown as number] as unknown as number | undefined;
    if (propId == null) {
      throw new Error(`missing property ${key} in ${node.metatype}`);
    } else if (typeof propId != "number") {
      throw new Error(`expected number, got ${typeof propId}: ${propId} from ${key} for ${describeNode(node)}`);
    }
    const prop = properties[propId];
    if (prop == null) throw new Error(`missing property info for ${node.metatype}.${propId} for ${describeNode(node)}`);
    const propType = getPropertyType(prop);
    const newValue = (update as any)[key];
    let operation: EditOperationData;
    const oldValuePacked = packValue((node as any)[key], propType);
    if (newValue == null) {
      operation = {
        metatype: ObjectType.EDIT_OPERATION,
        type: EditOperationType.CLEAR,
        path: [propId.toString()],
        oldValuePacked,
      };
    } else {
      const newValuePacked = packValue(newValue, propType);
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
        newValue = unpackValue(operation.newValuePacked!, valueType);
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
      (edit.type == EditType.UNARCHIVE && !graph.has(edit.nodePtr!)) ||
      (edit.type == EditType.RESTORE && !graph.has(edit.nodePtr!))
    ) {
      // add
      if (edit.nodeData == null) throw new Error(`missing node in edit: ${describeEdit(edit)}`);
      const node = unwrapSomeNode(edit.nodeData);
      // implicit metadata
      if (edit.type == EditType.CREATE || edit.type == EditType.UPSERT) {
        node.createdAt = edit.editedAt;
        node.createdByPtr = edit.subjectPtr;
      }
      node.updatedAt = edit.editedAt;
      node.deletedAt = undefined;
      node.updatedByPtr = edit.subjectPtr;
      if (
        node.parentPtr != null &&
        !graph.has(node.parentPtr) &&
        (options?.base == null || !options.base.has(node.parentPtr))
      ) {
        // NOTE :Robustness: ignoring new nodes with missing parents seems right but may be wonky :RichGraph
        if (options?.ignoreMissing) {
          continue;
        }
        throw new Error(
          `missing parent ${describeNode(node.parentPtr)} for ${describeNode(node)} in ${graph.describeSelf()}`,
        );
      }
      if (edit.type == EditType.CREATE || !graph.has(node)) {
        graph.add(node);
      } else {
        graph.update(node);
      }
      // extend setPaths for overlay
      if (options?.base != null) {
        if ((node as PartialNode<any>).setPaths == null) {
          (node as PartialNode<any>).setPaths = [...IMPLICIT_UPDATE_PROPERTIES_IDS.map((p) => [p.toString()])];
        }
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
      if (edit.type == EditType.UNARCHIVE || edit.type == EditType.RESTORE) {
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
      updatedNode.updatedByPtr = edit.subjectPtr;
      if (edit.type == EditType.ARCHIVE) {
        updatedNode.archivedAt = edit.editedAt;
      } else if (edit.type == EditType.UNARCHIVE) {
        updatedNode.archivedAt = undefined;
      } else if (edit.type == EditType.DELETE || edit.type == EditType.ERASE) {
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

type CommitFailure = { id: string; edits: EditData[]; error: RpcError };
type BufferCallback = (
  event:
    | {
        type: "add";
        meta: TransactionMeta;
        bufferedEdits: EditData[];
        connectionIdByEditId: Record<string, number>;
        debounce: DebounceLevel | null;
      }
    | {
        type: "reset";
        meta: TransactionMeta;
        bufferedEdits: EditData[];
        connectionIdByEditId: Record<string, number>;
        oldEdits: EditData[];
      },
) => void;
type AcceptedCallback = (event: {
  edits: EditData[];
  cascadedEdits: EditData[];
  connectionIdByEditId: Record<string, number>;
}) => void;

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
  /** Subscribes to *accepted* edits from this buffer */
  subscribeAccepted(sub: AcceptedCallback): () => void;
  /** Accepts the given edits from an external source (does not trigger commit, just to mark them as successfully committed) */
  onAccepted(edits: EditData[], cascadedEdits: EditData[]): void;
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

  subscribeAccepted(sub: AcceptedCallback): () => void {
    // nothing to do
    return () => {};
  }

  getBufferByConnection(): Record<number, EditData[]> {
    return {};
  }

  onAccepted(edits: EditData[], cascadedEdits: EditData[]): void {
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
  private acceptedSubs: Array<AcceptedCallback> = [];
  private currentTx: Transaction | null;
  private committingTx: Transaction | null;
  private bufferedEditsById: Record<string, EditData> = {};
  private bufferedConnectionByEditId: Record<string, number> = {};
  failedCommits: Ref<Record<string, CommitFailure>> = shallowRef({});

  constructor(id: number, scope: GraphScopeData) {
    this.id = id;
    this.scope = scope;
    this.currentTx = null;
    this.committingTx = null;
    this.reset();
  }

  get tx(): Transaction {
    if (this.currentTx == null) throw new Error("no active transaction");
    return this.currentTx;
  }

  async commit() {
    if (this.currentTx == null) throw new Error("no active transaction");
    if (this.committingTx != null) throw new Error(`transaction ${this.committingTx.id} is already committing`);

    const RECOVERABLE_ERRORS: StatusCode[] = [
      StatusCode.UNAVAILABLE,
      StatusCode.CANCELLED,
      StatusCode.DEADLINE_EXCEEDED,
    ];
    const RETRY_TIMEOUT = 1000;

    // swap
    this.committingTx = this.currentTx;
    this.currentTx = this._makeCurrentTx();

    // retry util we succeed or encounter a fatal error
    let attempt = 0;
    const client = getCachedGraphClient(this.scope);
    if (client == null) throw new Error(`no client for ${this.scope}`);
    while (true) {
      attempt++;
      try {
        log.trace("transaction.commit", {
          scope: this.scope,
          id: this.committingTx.id,
          edits: this.committingTx.edits,
        });

        // commit
        await client.commitTransaction(
          { edits: this.committingTx.edits, id: this.committingTx.id, scope: this.scope },
          { suppressErrors: true },
        );

        // success
        break;
      } catch (error) {
        // keep retrying
        if (RECOVERABLE_ERRORS.includes(StatusCode[(error as RpcError).code] as any)) {
          await new Promise((resolve) => setTimeout(resolve, RETRY_TIMEOUT));
          log.warn("transaction.commit.error.recoverable", { scope: this.scope, error });
          this.committingTx.edits.push(...this.currentTx.edits);
          continue;
        }

        // failed
        const fail: CommitFailure = {
          id: this.committingTx!.id,
          edits: this.committingTx!.edits,
          error: error as RpcError,
        };
        this.failedCommits.value[fail.id] = fail;
        triggerRef(this.failedCommits);

        // rollback
        log.error("transaction.commit.error.unrecoverable", { scope: this.scope, error });
        toaster.error({
          title: HUMANIZED_OPERATION_STATUS[(error as RpcError).code] ?? "Synchronization error",
          text: `Saving failed: ${IS_DEVELOPER_MODE.value ? (error as Error).message : (error as RpcError).code}`,
        });
        this.reset();

        // fatal error, stop retrying
        break;
      }
    }
    this.committingTx = null;
  }

  async reset() {
    this.currentTx = this._makeCurrentTx();
    const oldEdits = Object.values(this.bufferedEditsById);
    this.bufferedEditsById = {};
    this.bufferedConnectionByEditId = {};
    this.committingTx = null;
    this.bufferSubs.forEach((sub) =>
      sub({ type: "reset", meta: {}, connectionIdByEditId: {}, bufferedEdits: [], oldEdits }),
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
          bufferedEdits: [edit],
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

  subscribeAccepted(sub: AcceptedCallback): () => void {
    this.acceptedSubs.push(sub);
    return () => {
      const idx = this.acceptedSubs.indexOf(sub);
      if (idx >= 0) this.acceptedSubs.splice(idx, 1);
    };
  }

  onAccepted(edits: EditData[], cascadedEdits: EditData[]): void {
    // update buffer subscribers
    const connectionIdByEditId = { ...this.bufferedConnectionByEditId };
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
      this.bufferSubs.forEach((sub) => {
        sub({
          type: "reset",
          meta: {},
          bufferedEdits: newBufferedEdits,
          connectionIdByEditId: this.bufferedConnectionByEditId,
          oldEdits,
        });
      });
    }
    this.acceptedSubs.forEach((sub) => sub({ edits, cascadedEdits, connectionIdByEditId }));
  }

  togglePaused() {
    this.isPaused.value = !this.isPaused.value;
    log.trace("transaction.togglePaused", { scope: this.scope, paused: this.isPaused.value });
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
    return this.committingTx != null;
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
const txBuffersByDestackId: Ref<Record<string, RemoteTransactionBuffer>> = shallowRef({});
export const txBuffers = computed(() => getAllTransactionBuffers());

export function getAllTransactionBuffers(): TransactionBuffer[] {
  return [globalTxBuffer, ...Object.values(txBuffersByDestackId.value)];
}

/**
 * Gets the transaction buffer for the given scope (non-exclusively).
 * We maintain one transaction buffer per Destack and one for other universal nodes (outside of Destackes).
 * */
export function getTransactionBuffer(scope: GraphScopeData): TransactionBuffer {
  if (scope.destackId) {
    if (!txBuffersByDestackId.value[scope.destackId]) {
      const buffer = new RemoteTransactionBuffer(newBufferId(), scope);
      txBuffersByDestackId.value[scope.destackId] = buffer;
      triggerRef(txBuffersByDestackId);
      watchTransactionBuffer(buffer);
    }
    return txBuffersByDestackId.value[scope.destackId];
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
    if (event.type == "reset") return; // ignore resets
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
        if (buffer.isDirty) {
          if (userPtr.value == null) {
            log.warn("transaction.commit.discard");
          } else {
            buffer.commit();
          }
        }
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
export async function commitTransactionBuffers() {
  const buffers = [globalTxBuffer, ...Object.values(txBuffersByDestackId.value)];
  const commitPromises = [];
  for (const tx of buffers) {
    if (tx.isDirty && !tx.isCommitting && !tx.isPaused.value) {
      const ret = tx.commit();
      if (ret instanceof Promise) commitPromises.push(ret);
    }
  }
  await Promise.all(commitPromises);
}

/** Resets all transaction buffers. */
export function resetTransactionBuffers() {
  globalTxBuffer.reset();
  for (const tx of Object.values(txBuffersByDestackId.value)) {
    tx.reset();
  }
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

/** Gets the default transaction options for a value type */

const TRANSACTION_OPTIONS_TICK: TransactionOptions = { debounce: "tick" };
const TRANSACTION_OPTIONS_SHORT: TransactionOptions = { debounce: "short" };
const TRANSACTION_OPTIONS_LONG: TransactionOptions = { debounce: "long" };

const DEBOUNCE_TICK_TYPES: Set<PrimitiveType | DestackType> = new Set([PrimitiveType.BOOLEAN]);
const DEBOUNCE_LONG_TYPES: Set<PrimitiveType | DestackType> = new Set([
  DestackType.TEXT,
  DestackType.CODE,
  DestackType.OFFSET,
  DestackType.TRANSFORM,
  DestackType.VECTOR2,
  DestackType.VECTOR3,
  DestackType.VECTOR4,
  DestackType.LINE,
  DestackType.RECTANGLE,
  DestackType.RECTANGLE_CONSTRAINT,
]);

export function getTransactionOptionsForType(type: TypeIdentity): TransactionOptions {
  if (DEBOUNCE_TICK_TYPES.has(type.primitiveType!) || DEBOUNCE_TICK_TYPES.has(type.destackType!)) {
    return TRANSACTION_OPTIONS_TICK;
  } else if (DEBOUNCE_LONG_TYPES.has(type.primitiveType!) || DEBOUNCE_LONG_TYPES.has(type.destackType!)) {
    return TRANSACTION_OPTIONS_LONG;
  } else {
    return TRANSACTION_OPTIONS_SHORT;
  }
}
