import { getHostClient, supervisor, type OperationMetadata } from "@/proto/services";
import {
  BenchType,
  EditType,
  GraphScope,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  Timestamp,
  type AnyNodeData,
  type AnyPropertyType,
  type EditData,
  type IGraphIOClient,
  type NodeTypeMapping,
} from "@/proto/wire";
import { getDefaultProtoValue, makeNode, newStructId, unwrapSomeNode, wrapSomeNode } from "@/proto/wiring";
import { userPtr } from "@/system/client";
import { NodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { log } from "@/utils/log";
import { toValueRef } from "@/utils/ref";
import { v4 } from "uuid";
import { ref, watch, type Ref } from "vue";

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
  update<T extends NodeType>(
    update: Partial<Omit<NodeTypeMapping[T], "metatype">> & { metatype: T },
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
   */
  delete(node: AnyNodeData): void;
};

export class TransactionBuilder implements Transaction {
  scope: GraphScope;
  id: string;
  subject: NodeReferenceData | null;
  edits: EditData[] = [];
  subs: Array<(edit: EditData, debounced: boolean) => void> = [];

  constructor(scope: GraphScope, id: string, subject: NodeReferenceData | null) {
    this.scope = scope;
    this.id = id;
    this.subject = subject;
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
    const edit: EditData = {
      id: newStructId(),
      type,
      node: wrapSomeNode(node),
      nodeType: node.metatype as unknown as NodeType,
      properties: properties ?? [],
      subject: this.subject ?? undefined,
      scope: {
        benchId: "packagePtr" in node ? node.packagePtr?.benchId : undefined,
        packageId: "packagePtr" in node ? node.packagePtr?.id : undefined,
        transactionId: this.id,
      },
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
    // TODO :Cleanup: why doesn't Transaction.nodeIn typecheck properly (below for makeNode)?
    const node: NodeTypeMapping[T] =
      nodeIn.id == null ? makeNode(nodeIn as any) : (nodeIn as unknown as NodeTypeMapping[T]);
    this._addEdit(EditType.CREATE, node);
    return node as NodeTypeMapping[T];
  }

  upsert(node: AnyNodeData) {
    this._addEdit(EditType.UPSERT, node);
  }

  update<T extends NodeType>(
    update: Partial<Omit<NodeTypeMapping[T], "metatype">> & { metatype: T },
    options?: { debounce?: boolean },
  ) {
    const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[update.metatype as unknown as NodeType]!;
    const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[update.metatype as unknown as BenchType]!;
    const properties: number[] = [];
    const patchedNode = { ...update };
    let ord = 0;
    for (const propName of Object.keys(allProperties)) {
      if (!Number.isNaN(Number(propName))) continue; // skip numeric keys
      if (propName === "id" || propName === "metatype") {
        // keep as is (but not part of the 'update')
      } else if (propName == "parentPtr" || propName == "archivedAt" || propName == "deletedAt") {
        // ignore, cannot be updated directly - error?
      } else if (Object.prototype.hasOwnProperty.call(update, propName)) {
        // update the assigned property
        properties.push((allProperties as any)[propName]);
      } else {
        // init unset fields with an allowed default value
        //  (will be ignored anyway since its not in 'properties', but required for protobuf validation)
        (patchedNode as any)[propName] = getDefaultProtoValue(messageType.fields[ord]);
      }
      ord += 1;
    }
    this._addEdit(
      EditType.UPDATE,
      patchedNode as unknown as NodeTypeMapping[T],
      properties,
      options?.debounce ?? false,
    );
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
      node.deletedAt = now;
    }
  }
}

/**
 * Applies the edits to the graph (in place!).
 * If a 'base' graph is provided, the given graph is edited as an overlay.
 */
export function editGraph(
  graph: ReadNodeGraph & WriteNodeGraph,
  edits: EditData[],
  options?: { base?: ReadNodeGraph },
) {
  for (const edit of edits) {
    if (edit.node == null) throw new Error(`missing node in edit: ${edit}`);
    const nodeData = unwrapSomeNode(edit.node);
    if (edit.type == EditType.CREATE || (edit.type == EditType.UPSERT && !graph.get({ id: nodeData.id }))) {
      graph.add(nodeData);
    } else if (edit.type == EditType.DELETE) {
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
      } else if (edit.type == EditType.SOFT_DELETE || edit.type == EditType.RESTORE) {
        properties = [nodeProperties.deletedAt];
      } else {
        throw new Error(`unexpected edit type: ${edit.type}`);
      }
      const existingNode = graph.get({ id: nodeData.id }) as Readonly<AnyNodeData> | undefined;
      if (!existingNode) throw new Error(`missing node for update: ${nodeData.id}`);
      const updatedNode = { ...existingNode }; // clone
      for (const propId of properties) {
        const propName = nodeProperties[propId];
        (updatedNode as any)[propName] = (nodeData as any)[propName];
      }
      if (options?.base != null) {
        // overlay: update 'setProperties' with newly set properties
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
  readonly tx: Transaction;
  readonly overlay: ReadNodeGraph;

  /** Commits the current transaction. */
  commit(): void;
  /** Resets the current transaction and overlay. */
  reset(): void;
  /** Whether there are any pending uncommitted edits  */
  get isDirty(): boolean;
  /** Whether there are any pending commits */
  get isCommitting(): boolean;
  /** Whether transactions are currently processed */
  readonly isPaused: Readonly<Ref<boolean>>;

  togglePaused(): void;
}

/**
 * Applies transactions immediately to the graph.
 */
export class ImmediateTransactionBuffer implements TransactionBuffer {
  public readonly scope: GraphScope;
  public readonly graph: ReadNodeGraph & WriteNodeGraph;
  public readonly overlay: ReadNodeGraph;
  public readonly isPaused: Ref<boolean> = ref(false);
  private currentTx: TransactionBuilder | null = null; // always keep a single transaction

  constructor(scope: GraphScope, graph: ReadNodeGraph & WriteNodeGraph) {
    this.scope = scope;
    this.graph = graph;
    this.overlay = new NodeGraph({ scope, isPartial: true }); // just leave it empty since we apply immediately
    this.reset();
  }

  get tx(): Transaction {
    return this.currentTx!; // set in constructor
  }

  togglePaused() {
    this.isPaused.value = !this.isPaused.value;
    log.debug("transaction.togglePaused", { scope: this.scope, paused: this.isPaused.value });
  }

  commit() {
    // nothing to do
  }

  reset() {
    const newTx = new TransactionBuilder(this.scope, v4(), userPtr.value);
    // immediately apply and reset the transaction
    newTx.subscribe((edit) => {
      if (this.currentTx !== newTx) throw new Error("transaction is closed");
      canonicalizeEdits(Timestamp.now(), [edit]);
      editGraph(this.graph, [edit]);
      newTx.edits.length = 0;
    });
    this.currentTx = newTx;
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
export class SwapTransactionBuffer implements TransactionBuffer {
  public readonly scope: GraphScope;
  public readonly client: IGraphIOClient;
  public readonly overlay: NodeGraph;
  public readonly isPaused: Ref<boolean> = ref(false);
  private currentTx: Transaction | null;
  private pendingTx: Transaction | null;

  constructor(scope: GraphScope, client: IGraphIOClient) {
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

  togglePaused() {
    this.isPaused.value = !this.isPaused.value;
  }

  async commit() {
    if (this.currentTx == null) throw new Error("no active transaction");
    if (this.pendingTx != null) throw new Error(`transaction ${this.pendingTx.describeSelf()} is already committing`);
    try {
      // swap & commit
      log.trace("transaction.commit", {
        scope: this.scope,
        id: this.currentTx.id,
        edits: this.currentTx.edits,
      });
      this.pendingTx = this.currentTx;
      this.currentTx = this.makeCurrentTx();
      // nocheckin: retry / add back to buffer on failure?
      await this.client.commitTransaction(
        {
          edits: this.pendingTx.edits,
          id: this.pendingTx.id,
          scope: this.scope,
        },
        { suppressErrors: true } as OperationMetadata,
      );
    } catch (e) {
      // rollback
      log.error("transaction.commit.error", { scope: this.scope, error: e });
      this.reset();
    } finally {
      this.pendingTx = null;
    }
  }

  async reset() {
    this.overlay.clear();
    this.currentTx = this.makeCurrentTx();
  }

  private makeCurrentTx() {
    const tx = new TransactionBuilder(this.scope, v4(), userPtr.value);
    tx.subscribe((edit) => {
      if (this.currentTx !== tx) throw new Error(`transaction ${tx.describeSelf()} is closed`);
      editGraph(this.overlay, [edit], { base: this.overlay });
    });
    return tx;
  }

  get isDirty() {
    return this.currentTx != null && this.currentTx.edits.length > 0;
  }

  get isCommitting() {
    return false;
  }
}
// nocheckin: track edit by origin (root) view? (for separate undo/redo)

const globalTxBuffer: TransactionBuffer = new SwapTransactionBuffer({}, supervisor);
const benchTxBuffers: Record<string, SwapTransactionBuffer> = {};

/** Gets the transaction buffer for the given scope (non-exclusively). */
export async function getTransactionBuffer(scope: GraphScope): Promise<TransactionBuffer> {
  if (scope.benchId) {
    if (!benchTxBuffers[scope.benchId]) {
      const client = await getHostClient({ id: scope.benchId });
      benchTxBuffers[scope.benchId] = new SwapTransactionBuffer(scope, client);
    }
    return benchTxBuffers[scope.benchId];
  } else {
    return globalTxBuffer;
  }
}

/** Commits any pending transaction buffers. */
export function flushTransactionBuffers() {
  const buffers = [globalTxBuffer, ...Object.values(benchTxBuffers)];
  for (const tx of buffers) {
    if (tx.isDirty && !tx.isCommitting && !tx.isPaused.value) {
      tx.commit();
    }
  }
}

let _setupTransactionManagement = false;
export function setupTransactionManagement() {
  if (_setupTransactionManagement) return;
  // commit periodically
  setInterval(() => flushTransactionBuffers(), 1000);
  // commit on user change
  watch(toValueRef(userPtr), () => flushTransactionBuffers());
  // commit before exit
  window.addEventListener("beforeunload", (e) => flushTransactionBuffers());
  _setupTransactionManagement = true;
}
