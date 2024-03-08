import {
  EditType,
  NodeType,
  type AnyNodeData,
  type EditData,
  NODE_PROPERTY_ENUM_BY_TYPE,
  BenchType,
  type NodeTypeMapping,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  type AnyPropertyType,
  GraphScope,
} from "@/proto/wire";
import { newStructId, unwrapSomeNode, wrapSomeNode } from "@/proto/wiring";
import { packageIdByBenchId } from "@/system/global";
import { type ObservableNodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { v4 } from "uuid";
import { computed, getCurrentInstance, inject, ref, type Ref } from "vue";

export type TransactionBase = {
  /** Create a new node */
  create(node: AnyNodeData): void;
  /** Create or update all properties in the node */
  upsert(node: AnyNodeData): void;
  /**
   * Update regular properties in this node. If we already have an update for this node, extend that
   * TODO :Broken: handle debounced updates
   */
  update<T extends NodeType>(
    update: Partial<Omit<NodeTypeMapping[T], "metatype">> & { metatype: T; debounced?: boolean },
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
   * @deprecated use softDelete by default (not really deprecated, just to make it clear this should be used deliberately)
   */
  delete(node: AnyNodeData): void;
};

/** A transaction on the Bench state graph. */
export class Transaction implements TransactionBase {
  id: string;
  edits: EditData[] = [];

  constructor(id: string | undefined = undefined) {
    this.id = id ?? v4();
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

  _addEdit(type: EditType, node: AnyNodeData, properties?: number[]) {
    const edit = this._makeEdit(type, node, properties);
    this.edits.push(edit);
  }

  create(node: AnyNodeData) {
    this._addEdit(EditType.CREATE, node);
  }

  upsert(node: AnyNodeData) {
    this._addEdit(EditType.UPSERT, node);
  }

  update<T extends NodeType>(
    update: Partial<Omit<NodeTypeMapping[T], "metatype">> & { metatype: T; debounced?: boolean },
  ) {
    const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[update.metatype as unknown as NodeType]!;
    const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[update.metatype as unknown as BenchType]!;
    const properties: number[] = [];
    const patchedNode = { ...update };
    let ord = 0;
    for (const propName in Object.keys(allProperties)) {
      if (!Number.isNaN(Number(propName))) continue; // skip numeric keys
      if (propName === "id" || propName === "metatype") {
        // keep as is (but not part of the 'update')
      } else if ((update as any)[propName] !== undefined) {
        // update the assigned property
        properties.push((allProperties as any)[propName]);
      } else {
        // init unset fields with an allowed default value
        //  (will be ignored anyway since its not in 'properties', but required for protobuf validation)
        const field = messageType.fields[ord];
        (patchedNode as any)[propName] = field.repeat ? [] : undefined;
      }
      ord += 1;
    }
    this._addEdit(EditType.UPDATE, patchedNode as unknown as NodeTypeMapping[T], properties);
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

export function editGraph(graph: ReadNodeGraph & WriteNodeGraph, edits: EditData[]) {
  /** Applies the edits to the graph (in place!). Ignores soft deletion & archivation. */

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
      if (editType == EditType.UPDATE || editType == EditType.MOVE) {
        properties = edit.properties;
      } else if (editType == EditType.ARCHIVE || editType == EditType.UNARCHIVE) {
        properties = [nodeProperties.archivedAt];
      } else if (editType == EditType.SOFT_DELETE || editType == EditType.RESTORE) {
        properties = [nodeProperties.deletedAt];
      } else {
        throw new Error(`unexpected edit type: ${editType}`);
      }
      const existingNode = graph.get({ id: nodeData.id });
      if (!existingNode) throw new Error(`missing node for update: ${edit}`);
      for (const propId of properties) {
        const propName = nodeProperties[propId];
        (existingNode as any)[propName] = (nodeData as any)[propName];
      }
      graph.update(existingNode);
    }
  }
}

export function editGraphOverlay(base: ReadNodeGraph, overlay: ReadNodeGraph & WriteNodeGraph, edits: EditData[]) {
  /** Apply the given edits to an 'optimistic' overlay of a graph (using setProperties for updates). */

  throw new Error("not yet implemented");
}

/**
 * Buffer edits for a Transaction in some scope.
 */
export class TransactionBuffer {
  public scope: GraphScope;
  public currentTx: Transaction | null;
  public pendingTx: Transaction | null;

  constructor(scope: GraphScope) {
    this.scope = scope;
    this.currentTx = new Transaction();
    this.pendingTx = null;
  }

  public get tx(): Transaction {
    if (this.currentTx == null) throw new Error("no current transaction");
    return this.currentTx;
  }

  create(node: AnyNodeData) {
    this.tx.create(node);
  }

  upsert(node: AnyNodeData) {
    this.tx.upsert(node);
  }

  update<T extends NodeType>(
    update: Partial<Omit<NodeTypeMapping[T], "metatype">> & { metatype: T; debounced?: boolean },
  ) {
    this.tx.update(update);
  }

  move(node: AnyNodeData) {
    this.tx.move(node);
  }

  archive(node: AnyNodeData) {
    this.tx.archive(node);
  }

  unarchive(node: AnyNodeData) {
    this.tx.unarchive(node);
  }

  softDelete(node: AnyNodeData) {
    this.tx.softDelete(node);
  }

  restore(node: AnyNodeData) {
    this.tx.restore(node);
  }

  delete(node: AnyNodeData) {
    this.tx.delete(node);
  }

  commit() {
    if (this.currentTx == null) throw new Error("no current transaction");
    this.pendingTx = this.currentTx;
    this.currentTx = new Transaction();
  }
}

/**
 * The component-level context for graph operations (read & write).
 */
export type GraphContext = {
  scopeByBenchId: Ref<{ [benchId: string]: GraphScope }>;
  packageIdByBenchId: Ref<{ [benchId: string]: string }>;
  optimisticGraphs: Ref<ObservableNodeGraph[]>;
};

export const GLOBAL_GRAPH_CONTEXT: GraphContext = {
  packageIdByBenchId: computed(() => packageIdByBenchId.value),
  scopeByBenchId: computed(() => {
    const scopeByBenchId: { [benchId: string]: GraphScope } = {};
    for (const benchId of Object.keys(packageIdByBenchId.value)) {
      scopeByBenchId[benchId] = { benchId, packageId: packageIdByBenchId.value[benchId] };
    }
    return scopeByBenchId;
  }),
  optimisticGraphs: ref([]),
};

export const GRAPH_CONTEXT_KEY = Symbol("graphContext");

export function useGraphContext(): GraphContext {
  const component = getCurrentInstance();
  if (component == null) {
    return GLOBAL_GRAPH_CONTEXT;
  } else {
    const localContext = inject(GRAPH_CONTEXT_KEY, null);
    if (localContext != null) {
      return localContext;
    } else {
      return GLOBAL_GRAPH_CONTEXT;
    }
  }
}

export const txBuffersByBench: Ref<{ [benchId: string]: TransactionBuffer[] }> = ref({});
