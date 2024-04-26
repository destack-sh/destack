import { HUMANIZED_OPERATION_STATUS, getHostClient, supervisor } from "@/proto/services";
import {
  CommitTransactionRequest,
  EditType,
  GraphScope,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  Timestamp,
  type AnyNodeData,
  type AnyPropertyType,
  type EditData,
  type IGraphIOClient,
  type NodeTypeMapping,
} from "@/proto/wire";
import {
  describeNode,
  makeNode,
  nodeReference,
  unwrapSomeNode,
  wrapSomeNode,
  type AnyNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { nonce, origin, userPtr } from "@/system/client";
import { type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { makeIcon } from "@/system/icon";
import { toaster } from "@/system/toast";
import { AsyncEvent } from "@/utils/functools";
import { IS_DEBUG, TRANSACTION_FLUSH_INTERVAL } from "@/utils/globals";
import { log } from "@/utils/log";
import { toValueRef } from "@/utils/ref";
import { uuidt } from "@/utils/uuidt";
import type { RpcError } from "grpc-web";
import { ref, shallowRef, triggerRef, watch, type Ref } from "vue";

const CONSTANT_PROPERTIES = ["metatype", "id", "ck"];
const CONSTANT_IN_UPDATE_PROPERTIES = [...CONSTANT_PROPERTIES, "parentPtr", "archivedAt", "deletedAt"];

const nonce8BytesPostfix = nonce.replace("-", "").slice(0, 16);

function newEditId(): string {
  return uuidt({ nonce: nonce8BytesPostfix });
}

function newTransactionId(): string {
  return uuidt({ nonce: nonce8BytesPostfix });
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
  update<T extends AnyNodeData>(
    node: T,
    update: Partial<T> | (keyof Omit<T, "metatype" | "id" | "ck">)[],
    options?: { debounce?: boolean },
  ): void;
  /** Convenience debounced update. */
  updateDebounced<T extends AnyNodeData>(
    node: T,
    update: Partial<T> | (keyof Omit<T, "metatype" | "id" | "ck">)[],
  ): void;
  /** Move node between parents */
  move(node: AnyNodeData, parentPtr?: AnyNodeReferenceData): void;

  /** Archive node (incl. descendants) */
  archive(node: AnyNodeData): void;
  /** Restore node from archive (incl. descendants) */
  unarchive(node: AnyNodeData): void;

  /** Soft delete node (incl. descendants), marked for later deletion after retention period */
  softDelete(node: AnyNodeData): void;
  /** Restore node from soft delete */
  restore(node: AnyNodeData): void;
  /**
   * @deprecated use softDelete (not actually deprecated, but to be used deliberately)
   * also NOTE: Transaction.delete is not handled optimistically in our transaction buffer overlays
   */
  delete(node: AnyNodeData): void;
};

export class TransactionBuilder implements Transaction {
  public readonly scope: GraphScope;
  public readonly id: string;
  public readonly subject: NodeReferenceData | null;
  public readonly edits: EditData[] = [];
  private readonly debouncedUpdates: Record<string, EditData> = {};
  private subs: Array<(edit: EditData, debounced: boolean) => void> = [];
  private benchPtr: TypedNodeReferenceData<NodeType.BENCH> | null;

  constructor(scope: GraphScope, id: string, subject: NodeReferenceData | null) {
    this.scope = scope;
    this.id = id;
    this.subject = subject;
    this.benchPtr = scope.benchId != null ? nodeReference(NodeType.BENCH, scope.benchId) : null;
  }

  describeSelf(): string {
    return `Transaction(${this.id}, ${this.edits.length} edits)`;
  }

  onEdit(sub: (edit: EditData, debounced: boolean) => void): () => void {
    this.subs.push(sub);
    return () => {
      const idx = this.subs.indexOf(sub);
      if (idx >= 0) this.subs.splice(idx, 1);
    };
  }

  _makeEdit(type: EditType, node: AnyNodeData, properties?: number[]): EditData {
    const benchId = (node as any).benchPtr?.id ?? this.scope.benchId;
    const packageId = (node as any).packagePtr?.id ?? this.scope.packageId;
    const allProperties = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
    if ("packagePtr" in allProperties && packageId == null)
      throw new Error(`missing packagePtr in ${describeNode(node)}`);

    const edit: EditData = {
      id: newEditId(),
      type,
      origin: origin.value,
      scope: {
        benchId,
        packageId,
        transactionId: this.id,
      },
      nodeType: node.metatype as unknown as NodeType,
      node: wrapSomeNode(node),
      properties: properties ?? [],
      subject: this.subject ?? undefined,
    };
    return edit;
  }

  _addNewEdit(type: EditType, node: AnyNodeData, properties?: number[], debounced: boolean = false) {
    const edit = this._makeEdit(type, node, properties);
    this.edits.push(edit);
    this._notifyEdit(edit, debounced);
    return edit;
  }

  _notifyEdit(edit: EditData, debounced: boolean) {
    for (const sub of this.subs) {
      sub(edit, debounced);
    }
  }

  addEdit(edit: EditData): void {
    this.edits.push(edit);
    this._notifyEdit(edit, false);
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

    this._addNewEdit(EditType.CREATE, node);
    return node as NodeTypeMapping[T];
  }

  upsert(node: AnyNodeData) {
    this._addNewEdit(EditType.UPSERT, { ...node });
  }

  update<T extends AnyNodeData>(
    node: T,
    update: Partial<T> | (keyof Omit<T, "metatype" | "id" | "ck">)[],
    options?: { debounce?: boolean },
  ) {
    const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype as unknown as NodeType]!;

    // map update values
    let propertiesNames: string[];
    if (Array.isArray(update)) {
      propertiesNames = update as string[];
      update = {}; // node is already updated
      propertiesNames.forEach((propName) => ((update as any)[propName] = (node as any)[propName]));
    } else if (typeof update == "object") {
      propertiesNames = Object.keys(update);
    } else {
      throw new Error(`unexpected update type: ${update}`);
    }

    // map properties
    const properties = [];
    for (const propName of propertiesNames) {
      if (CONSTANT_PROPERTIES.includes(propName))
        throw new Error(`cannot update constant property for ${describeNode(node)}: ${propName}`);
      const propId = (allProperties as any)[propName];
      if (propId == null) throw new Error(`missing property id for ${describeNode(node)}: ${propName as string}`);
      properties.push(propId);
    }

    if (options?.debounce && this.debouncedUpdates[node.id]) {
      // merge into existing edit & notify directly
      const debouncedEdit = this.debouncedUpdates[node.id];
      properties
        .filter((propId) => !debouncedEdit.properties.includes(propId))
        .forEach((i) => debouncedEdit.properties.push(i));
      const prevNode = unwrapSomeNode(debouncedEdit.node!);
      for (const propName of propertiesNames) {
        (prevNode as any)[propName] = (update as any)[propName];
      }
      debouncedEdit.node = wrapSomeNode(prevNode);
      this._notifyEdit(debouncedEdit, true);
    } else {
      // create new edit
      const patchedNode = { ...node, ...update } as T;
      const edit = this._addNewEdit(EditType.UPDATE, patchedNode, properties, options?.debounce ?? false);

      if (options?.debounce) {
        this.debouncedUpdates[node.id] = edit;
      }
    }
  }

  updateDebounced<T extends AnyNodeData>(node: T, update: Partial<T> | (keyof Omit<T, "metatype" | "id" | "ck">)[]) {
    this.update(node, update, { debounce: true });
  }

  move(node: AnyNodeData, parentPtr?: AnyNodeReferenceData) {
    if (parentPtr != null) node = { ...node, parentPtr };
    else node = { ...node };
    this._addNewEdit(EditType.MOVE, node);
  }

  archive(node: AnyNodeData) {
    this._addNewEdit(EditType.ARCHIVE, { ...node });
  }

  unarchive(node: AnyNodeData) {
    this._addNewEdit(EditType.UNARCHIVE, { ...node });
  }

  softDelete(node: AnyNodeData) {
    this._addNewEdit(EditType.SOFT_DELETE, { ...node });
  }

  restore(node: AnyNodeData) {
    this._addNewEdit(EditType.RESTORE, { ...node });
  }

  delete(node: AnyNodeData) {
    this._addNewEdit(EditType.DELETE, { ...node });
  }
}

/** 'Canonicalizes' edits by imputing tracking info (just like in host). See :EditCanonicalization. */
export function canonicalizeEdits(now: Timestamp, edits: EditData[]) {
  for (const edit of edits) {
    const node = unwrapSomeNode(edit.node!);
    if (edit.type == EditType.CREATE || edit.type == EditType.UPSERT) {
      node.createdAt = now;
      node.createdByPtr = edit.subject;
      node.updatedAt = now;
      node.updatedByPtr = edit.subject;
    } else if (edit.type == EditType.MOVE || edit.type == EditType.UPDATE) {
      node.updatedAt = now;
      node.updatedByPtr = edit.subject;
    } else if (edit.type == EditType.ARCHIVE) {
      node.archivedAt = now;
    } else if (edit.type == EditType.UNARCHIVE) {
      node.archivedAt = undefined;
    } else if (edit.type == EditType.SOFT_DELETE) {
      node.deletedAt = now;
    } else if (edit.type == EditType.RESTORE) {
      node.deletedAt = undefined;
    } else if (edit.type == EditType.DELETE) {
      node.deletedAt = now; // just pretend it's deleted
    }
  }
}

/**
 * Applies the edits to the graph (in place!).
 * If a 'base' graph is provided, the given graph is edited as an overlay.
 */
export function editGraph(graph: ReadNodeGraph & WriteNodeGraph, edits: EditData[], options?: { isOverlay: boolean }) {
  for (const edit of edits) {
    if (edit.node == null) throw new Error(`missing node in edit: ${edit}`);
    const nodeData = unwrapSomeNode(edit.node);
    if (edit.type == EditType.CREATE || (edit.type == EditType.UPSERT && !graph.get({ id: nodeData.id }))) {
      graph.add(nodeData);
    } else if (edit.type == EditType.DELETE && !options?.isOverlay) {
      graph.remove(nodeData);
    } else {
      let properties: number[];
      const nodeProperties = NODE_PROPERTY_ENUM_BY_TYPE[nodeData.metatype]!;
      if (edit.type == EditType.UPDATE || edit.type == EditType.UPSERT) {
        properties = edit.properties;
      } else if (edit.type == EditType.MOVE) {
        properties = [nodeProperties.parentPtr];
      } else if (edit.type == EditType.ARCHIVE || edit.type == EditType.UNARCHIVE) {
        properties = [nodeProperties.archivedAt];
      } else if (edit.type == EditType.SOFT_DELETE || edit.type == EditType.DELETE || edit.type == EditType.RESTORE) {
        properties = [nodeProperties.deletedAt];
      } else {
        throw new Error(`unexpected edit type: ${EditType[edit.type]}`);
      }

      let existingNode = graph.get({ id: nodeData.id }) as Readonly<Partial<AnyNodeData>> | undefined;
      if (!existingNode) {
        if (options?.isOverlay) existingNode = nodeData;
        else throw new Error(`missing node for ${EditType[edit.type]}: ${describeNode(nodeData)}`);
      }

      const updatedNode = { setProperties: [], ...existingNode } as AnyNodeData; // clone
      for (const propId of properties) {
        const propName = nodeProperties[propId];
        (updatedNode as any)[propName] = (nodeData as any)[propName];
      }
      if (options?.isOverlay) {
        // update 'setProperties' with newly set properties
        updatedNode.setProperties = [...(existingNode.setProperties ?? [])];
        properties
          .filter((propId) => !updatedNode.setProperties.includes(propId))
          .forEach((i) => updatedNode.setProperties.push(i));
      }

      graph.update(updatedNode);
    }
  }
}

// TODO :Incomplete: track edit by origin (root) view? (for separate undo/redo)

type CommitFailure = {
  id: string;
  edits: EditData[];
  error: RpcError;
};
type PendingCallback = (event: { type: "add"; edits: EditData[] } | { type: "reset"; edits: EditData[] }) => void;
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
  /** Retryable commits in case of error.  */
  readonly failedCommits?: Readonly<Ref<Record<string, CommitFailure>>>;

  /** Commits the current transaction. */
  commit(): void | Promise<void>;
  /** Resets the current transaction and overlay. */
  reset(): void | Promise<void>;
  /** Accepts the given edits from an external source (does not trigger onCommitted) */
  accept(edits: EditData[]): void;
  /** Subscribes to *pending* edits from this buffer */
  onPending(sub: PendingCallback): () => void;
  /** Subscribes to *confirmed* edits from this buffer */
  onCommitted(sub: CommittedCallback): () => void;
  /** Force retries the given commit (for debugging) */
  retry?(id: string): Promise<void>;

  /** Whether there are any pending uncommitted edits  */
  get isDirty(): boolean;
  /** Whether there are any pending commits */
  get isCommitting(): boolean;

  /** Whether transactions are currently processed (for debugging). */
  readonly isPaused: Readonly<Ref<boolean>>;
  /** Toggle automatic flushing for debugging. */
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
    const newTx = new TransactionBuilder(this.scope, uuidt({ nonce: nonce8BytesPostfix }), userPtr.value);
    // immediately apply and reset the transaction
    newTx.onEdit((edit) => {
      if (this.currentTx !== newTx) throw new Error("transaction is closed");
      // apply edit directly
      canonicalizeEdits(Timestamp.now(), [edit]);
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

  onPending(sub: PendingCallback): () => void {
    // nothing to do
    return () => {};
  }

  onCommitted(sub: CommittedCallback): () => void {
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
      this.currentTx = this.makeCurrentTx();

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
              this.currentTx = this.makeCurrentTx();
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
        text: `Saving ${this.pendingTx?.edits.length ?? 0} edits failed: ${IS_DEBUG ? (error as Error).message : (error as RpcError).code}`,
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
    this.currentTx = this.makeCurrentTx();
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

  private makeCurrentTx() {
    const tx = new TransactionBuilder(this.scope, newTransactionId(), userPtr.value);
    tx.onEdit((edit) => {
      if (this.currentTx !== tx) throw new Error(`transaction ${tx.describeSelf()} is closed`);
      this.pendingEditsById[edit.id] = edit;
      canonicalizeEdits(Timestamp.now(), [edit]);
      // directly update overlays since this edit is 'last' now (by definition)
      this.pendingSubs.forEach((sub) => sub({ type: "add", edits: [edit] }));
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
      // re-derive overlays from pending edits
      const newPendingEdits = Object.values(this.pendingEditsById);
      this.pendingSubs.forEach((sub) => sub({ type: "reset", edits: newPendingEdits }));
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
const benchTxBuffers: Ref<Record<string, RemoteTransactionBuffer>> = shallowRef({});
const benchTxBuffersLocks: Record<string, AsyncEvent> = {};

export function getAllTransactionBuffers(): TransactionBuffer[] {
  return [globalTxBuffer, ...Object.values(benchTxBuffers.value)];
}

/** Gets the transaction buffer for the given scope (non-exclusively). */
export async function getTransactionBuffer(scope: GraphScope): Promise<TransactionBuffer> {
  if (scope.benchId) {
    if (!benchTxBuffers.value[scope.benchId]) {
      // synchronize so that only one buffer is created per bench even when called concurrently
      if (!benchTxBuffersLocks[scope.benchId]) {
        benchTxBuffersLocks[scope.benchId] = new AsyncEvent();
      } else {
        await benchTxBuffersLocks[scope.benchId].wait();
      }
      if (!benchTxBuffers.value[scope.benchId]) {
        const client = await getHostClient({ id: scope.benchId });
        benchTxBuffers.value[scope.benchId] = new RemoteTransactionBuffer(newBufferId(), scope, client);
        triggerRef(benchTxBuffers);
        benchTxBuffersLocks[scope.benchId].set();
        delete benchTxBuffersLocks[scope.benchId];
      }
    }
    return benchTxBuffers.value[scope.benchId];
  } else {
    return globalTxBuffer;
  }
}

/** Commits any pending transactions in the current buffers. */
export async function flushTransactionBuffers(options: { force: boolean } = { force: true }) {
  const buffers = [globalTxBuffer, ...Object.values(benchTxBuffers.value)];
  const commitPromises = [];
  for (const tx of buffers) {
    if (tx.isDirty && !tx.isCommitting && (options.force || !tx.isPaused.value)) {
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
  // commit periodically
  // TODO :UX: tune transaction commit schedule (maybe commit more quickly after non-debounced edits?)
  let flushInterval: any | null = null;
  watch(
    TRANSACTION_FLUSH_INTERVAL,
    () => {
      if (flushInterval) clearInterval(flushInterval);
      flushInterval = setInterval(() => flushTransactionBuffers({ force: false }), TRANSACTION_FLUSH_INTERVAL.value);
    },
    { immediate: true },
  );
  // commit on user change
  watch(toValueRef(userPtr), () => flushTransactionBuffers({ force: false }));
  // commit before exit
  window.addEventListener("beforeunload", (e) => flushTransactionBuffers({ force: false }));
}
