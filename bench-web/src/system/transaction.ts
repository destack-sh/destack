import { supervisor } from "@/proto/services";
import {
  BenchType,
  EditType,
  GraphScope,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeType,
  type AnyNodeData,
  type AnyPropertyType,
  type EditData,
  type IGraphIOClient,
  type NodeTypeMapping,
  Timestamp,
} from "@/proto/wire";
import { getDefaultProtoValue, newStructId, unwrapSomeNode, wrapSomeNode } from "@/proto/wiring";
import { NodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { v4 } from "uuid";

/** A transaction on the Bench state graph. */
export type Transaction = {
  readonly scope: GraphScope;
  readonly id: string;
  readonly edits: EditData[];

  /** Create a new node */
  create(node: AnyNodeData): void;
  /** Create or update all properties in the node */
  upsert(node: AnyNodeData): void;
  /**
   * Update regular properties in this node. If we already have an update for this node, extend that
   * TODO :Broken: handle debounce updates
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
  edits: EditData[] = [];
  subs: Array<(edit: EditData, debounced: boolean) => void> = [];

  constructor(scope: GraphScope, id: string) {
    this.scope = scope;
    this.id = id;
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

  create(node: AnyNodeData) {
    this._addEdit(EditType.CREATE, node);
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

/** Applies the edits to the graph (in place!). Ignores soft deletion & archivation. */
export function editGraph(graph: ReadNodeGraph & WriteNodeGraph, edits: EditData[]) {
  for (const edit of edits) {
    if (edit.node == null) throw new Error(`missing node in edit: ${edit}`);
    const nodeData = unwrapSomeNode(edit.node);
    const editType = edit.type;
    if (editType == EditType.CREATE || (editType == EditType.UPSERT && !graph.get({ id: nodeData.id }))) {
      graph.add(nodeData);
    } else if (editType == EditType.DELETE) {
      graph.remove(nodeData);
    } else {
      let properties: number[];
      const nodeProperties = NODE_PROPERTY_ENUM_BY_TYPE[nodeData.metatype]!;
      if (editType == EditType.UPDATE) {
        properties = edit.properties;
      } else if (editType == EditType.MOVE) {
        properties = [nodeProperties.parentPtr];
      } else if (editType == EditType.ARCHIVE || editType == EditType.UNARCHIVE) {
        properties = [nodeProperties.archivedAt];
      } else if (editType == EditType.SOFT_DELETE || editType == EditType.RESTORE) {
        properties = [nodeProperties.deletedAt];
      } else {
        throw new Error(`unexpected edit type: ${editType}`);
      }
      let existingNode = graph.get({ id: nodeData.id });
      if (!existingNode) throw new Error(`missing node for update: ${nodeData.id}`);
      existingNode = { ...existingNode }; // clone
      for (const propId of properties) {
        const propName = nodeProperties[propId];
        (existingNode as any)[propName] = (nodeData as any)[propName];
      }
      graph.update(existingNode);
    }
  }
}

/** Apply the given edits to an 'optimistic' overlay of a graph (using setProperties for partial updates). */
export function editGraphOverlay(base: ReadNodeGraph, overlay: ReadNodeGraph & WriteNodeGraph, edits: EditData[]) {
  throw new Error("not yet implemented");
}

/**
 * A transaction buffer provides Transactions and applies them to the graph.
 */
export interface TransactionBuffer {
  tx: Transaction;
  overlay: ReadNodeGraph;
}

/**
 * Applies transactions immediately to the graph.
 */
export class ImmediateTransactionBuffer implements TransactionBuffer {
  public readonly scope: GraphScope;
  public readonly graph: ReadNodeGraph & WriteNodeGraph;
  public readonly overlay: ReadNodeGraph;
  public readonly tx: TransactionBuilder; // always keep a single transaction

  constructor(scope: GraphScope, graph: ReadNodeGraph & WriteNodeGraph) {
    this.scope = scope;
    this.graph = graph;
    this.overlay = new NodeGraph({ scope, isPartial: true }); // just leave it empty since we apply immediately
    this.tx = new TransactionBuilder(scope, v4());

    // immediately apply and reset the transaction
    this.tx.subscribe((edit) => {
      canonicalizeEdits(Timestamp.now(), [edit]);
      editGraph(this.graph, [edit]);
      this.tx.edits.length = 0;
    });
  }
}

/**
 * A buffer with a single active transaction that can be committed to a remote client.
 */
export class SwapTransactionBuffer implements TransactionBuffer {
  public readonly scope: GraphScope;
  public readonly client: IGraphIOClient;
  public readonly overlay: ReadNodeGraph;
  public currentTx: Transaction;
  public pendingTx: Transaction | null;

  constructor(scope: GraphScope, client: IGraphIOClient) {
    this.scope = scope;
    this.currentTx = new TransactionBuilder(scope, v4());
    this.pendingTx = null;
    this.overlay = new NodeGraph({ scope, isPartial: true });
    this.client = client;
  }

  get tx(): Transaction {
    return this.currentTx;
  }

  // nocheckin: commit/swap/overlay remote transaction buffer
}
// nocheckin: track edit by origin (root) view? (for separate undo/redo)

const globalTxBuffer: TransactionBuffer = new SwapTransactionBuffer({}, supervisor);
const benchTxBuffers: Record<string, SwapTransactionBuffer> = {};

/** Gets the transaction buffer for the given scope (non-exclusively). */
export function getTransactionBuffer(scope: GraphScope): TransactionBuffer {
  if (scope.benchId) {
    if (!benchTxBuffers[scope.benchId]) {
      benchTxBuffers[scope.benchId] = new SwapTransactionBuffer(scope, supervisor);
    }
    return benchTxBuffers[scope.benchId];
  } else {
    return globalTxBuffer;
  }
}
