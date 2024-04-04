import { HUMANIZED_OPERATION_STATUS, getHostClient, supervisor } from "@/proto/services";
import {
  BenchType,
  EditType,
  GraphScope,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
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
  getDefaultProtoValue,
  makeNode,
  nodeReference,
  unwrapSomeNode,
  wrapSomeNode,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { nonce, origin, userPtr } from "@/system/client";
import { NodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { toaster } from "@/system/toast";
import { AsyncEvent } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
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

  /** Create a new node */
  create<T extends NodeType>(
    node: { metatype: T | BenchType } & Partial<Omit<NodeTypeMapping[T], "metatype">>,
  ): NodeTypeMapping[T];
  /** Create or update all properties in the node */
  upsert(node: AnyNodeData): void;
  /**
   * Update regular properties in this node. If we already have an update for this node, update it.
   * nocheckin :Broken: handle debounce updates somehow?
   */
  update<T extends AnyNodeData>(
    node: T,
    update: Partial<T> | (keyof Omit<T, "metatype" | "id" | "ck">)[],
    options?: { debounce?: boolean },
  ): void;
  /** Move node between parents */
  move(node: AnyNodeData): void;
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

  subscribe(sub: (edit: EditData, debounced: boolean) => void): () => void {
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

  _addEdit(type: EditType, node: AnyNodeData, properties?: number[], debounced?: boolean) {
    const edit = this._makeEdit(type, node, properties);
    this.edits.push(edit);
    for (const sub of this.subs) {
      sub(edit, debounced ?? false);
    }
  }

  create<T extends NodeType>(
    nodeIn: { metatype: T | BenchType } & Partial<Omit<NodeTypeMapping[NodeType], "metatype">>,
  ): NodeTypeMapping[T] {
    // fill in scope
    const properties = PROPERTY_ENUM_BY_TYPE[nodeIn.metatype as unknown as BenchType]!;
    if ("packagePtr" in properties && (nodeIn as any).packagePtr == null) {
      throw new Error(`missing packagePtr in ${describeNode(nodeIn)}`); // can't infer package
    }
    if ("benchPtr" in properties) {
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

    this._addEdit(EditType.CREATE, node);
    return node as NodeTypeMapping[T];
  }

  upsert(node: AnyNodeData) {
    this._addEdit(EditType.UPSERT, node);
  }

  update<T extends AnyNodeData>(
    node: T,
    update: Partial<T> | (keyof Omit<T, "metatype" | "id" | "ck">)[],
    options?: { debounce?: boolean },
  ) {
    const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype as unknown as NodeType]!;
    const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[node.metatype as unknown as BenchType]!;

    // map update values
    let patchedNode: T;
    let propertiesNames: string[];
    if (Array.isArray(update)) {
      patchedNode = { ...node };
      propertiesNames = update as string[];
      for (const propName of update) {
        if (propName === "id" || propName === "metatype") {
          // keep as is (but not part of the 'update')
        } else if (propName == "parentPtr" || propName == "archivedAt" || propName == "deletedAt") {
          // ignore, cannot be updated directly - error?
        } else {
          const propId = (allProperties as any)[propName];
          if (propId == null) throw new Error(`missing property id for ${propName as string}`);
          const value = getDefaultProtoValue(messageType.fields[propId]);
          if (value !== undefined) (patchedNode as any)[propName] = value;
        }
      }
    } else if (typeof update == "object") {
      patchedNode = { ...node, ...update };
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

    this._addEdit(EditType.UPDATE, patchedNode, properties, options?.debounce ?? false);
  }

  move(node: AnyNodeData) {
    this._addEdit(EditType.MOVE, node);
  }

  archive(node: AnyNodeData) {
    this._addEdit(EditType.ARCHIVE, node);
  }

  unarchive(node: AnyNodeData) {
    this._addEdit(EditType.UNARCHIVE, node);
  }

  softDelete(node: AnyNodeData) {
    this._addEdit(EditType.SOFT_DELETE, node);
  }

  restore(node: AnyNodeData) {
    this._addEdit(EditType.RESTORE, node);
  }

  delete(node: AnyNodeData) {
    this._addEdit(EditType.DELETE, node);
  }
}

/** 'Canonicalizes' edits by imputing tracking info (just like in host). */
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

/**
 * A transaction buffer provides Transactions and applies them to the graph.
 */
export interface TransactionBuffer {
  readonly id: number;
  /** Current active Transaction. */
  readonly tx: Transaction;
  /** Overlay of unconfirmed edits in active or pending transactions. */
  readonly overlay: ReadNodeGraph;

  /** Commits the current transaction. */
  commit(): void | Promise<void>;
  /** Resets the current transaction and overlay. */
  reset(): void | Promise<void>;
  /** Accepts the given edits from an external source (does not trigger onAccepted) */
  accept(edits: EditData[]): void;
  /** Subscribes to *confirmed* edits from this buffer */
  onAccepted(sub: (edits: EditData[]) => void): () => void;
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
  public readonly overlay: ReadNodeGraph;
  public readonly isPaused: Ref<boolean> = ref(false);
  private acceptedSubs: Array<(edits: EditData[]) => void> = [];
  private currentTx: TransactionBuilder | null = null; // always keep a single transaction

  constructor(id: number, scope: GraphScope, graph: ReadNodeGraph & WriteNodeGraph) {
    this.id = id;
    this.scope = scope;
    this.graph = graph;
    this.overlay = new NodeGraph({ scope, isPartial: true }); // just leave it empty since we apply immediately
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
    newTx.subscribe((edit) => {
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

  onAccepted(sub: (edits: EditData[]) => void): () => void {
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
  public readonly overlay: NodeGraph;
  public readonly isPaused: Ref<boolean> = ref(false);
  private acceptedSubs: Array<(edits: EditData[]) => void> = [];
  private currentTx: Transaction | null;
  private pendingTx: Transaction | null;
  private pendingEdits: Record<string, EditData> = {};

  constructor(id: number, scope: GraphScope, client: IGraphIOClient) {
    this.id = id;
    this.scope = scope;
    this.currentTx = null;
    this.pendingTx = null;
    this.overlay = new NodeGraph({ scope, isPartial: true });
    this.client = client;
    this.reset();
  }

  get tx(): Transaction {
    if (this.currentTx == null) throw new Error("no active transaction");
    return this.currentTx;
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
        { suppressErrors: true, retry: true },
      );
      for (let i = 0; i < edits.length; i++) {
        edits[i].revision = revisions[i];
      }

      // notify on success
      this.acceptedSubs.forEach((sub) => sub(edits));
    } catch (error) {
      // rollback
      log.error("transaction.commit.error", { scope: this.scope, error });
      toaster.error({
        title: HUMANIZED_OPERATION_STATUS[(error as RpcError).code] ?? "Synchronization error",
        text: `Saving ${this.pendingTx?.edits.length ?? 0} edits failed: ${IS_DEBUG ? (error as Error).message : (error as RpcError).code}`,
      });
      this.reset();
    } finally {
      this.pendingTx = null;
    }
  }

  async reset() {
    this.currentTx = this.makeCurrentTx();
    this.pendingEdits = {};
    this.pendingTx = null;
    this.overlay.clear();
  }

  private makeCurrentTx() {
    const tx = new TransactionBuilder(this.scope, newTransactionId(), userPtr.value);
    tx.subscribe((edit) => {
      if (this.currentTx !== tx) throw new Error(`transaction ${tx.describeSelf()} is closed`);
      this.pendingEdits[edit.id] = edit;
      canonicalizeEdits(Timestamp.now(), [edit]);
      // directly update overlay since we know this edit is 'last'
      editGraph(this.overlay, [edit], { isOverlay: true });
      console.log("edit", Object.values(this.pendingEdits).length, edit); // nocheckin
    });
    return tx;
  }

  accept(edits: EditData[]): void {
    let pendingEditsChanged = false;
    for (const edit of edits) {
      if (this.pendingEdits[edit.id]) {
        delete this.pendingEdits[edit.id];
        pendingEditsChanged = true;
      }
    }
    if (pendingEditsChanged) {
      // re-derive overlay from pending edits
      this.overlay.clear();
      editGraph(this.overlay, Object.values(this.pendingEdits), { isOverlay: true });
      console.log("accept", Object.values(this.pendingEdits).length); // nocheckin
    }
  }

  onAccepted(sub: (edits: EditData[]) => void): () => void {
    this.acceptedSubs.push(sub);
    return () => {
      const idx = this.acceptedSubs.indexOf(sub);
      if (idx >= 0) this.acceptedSubs.splice(idx, 1);
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
// nocheckin: track edit by origin (root) view? (for separate undo/redo)

let bufferId = 0;
export function newBufferId() {
  return bufferId++;
}
const globalTxBuffer: TransactionBuffer = new RemoteTransactionBuffer(newBufferId(), {}, supervisor);
const benchTxBuffers: Ref<Record<string, RemoteTransactionBuffer>> = shallowRef({});
const benchTxBuffersLocks: Record<string, AsyncEvent> = {};

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
  setInterval(() => flushTransactionBuffers({ force: false }), 1000);
  // commit on user change
  watch(toValueRef(userPtr), () => flushTransactionBuffers({ force: false }));
  // commit before exit
  window.addEventListener("beforeunload", (e) => flushTransactionBuffers({ force: false }));
}
