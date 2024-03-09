import {
  StructType,
  type GraphScope,
  type IGraphIOClient,
  type NodeReferenceData,
  type NodeType,
  type ReadOptionsData,
  type NodeTypeMapping,
  ReadType,
  AggregationData,
} from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import { accessAsOwner, type AccessArbiter, READ_TYPES } from "@/system/access";
import { LOCAL_BENCH_ID } from "@/system/global";
import { ProxyNodeGraph, type ReadNodeGraph } from "@/system/graph";
import type { TransactionBuffer } from "@/system/transaction";
import type { SubRef } from "@/utils/ref";
import { toRef, type MaybeRef, type Ref, shallowRef, type ShallowRef, isRef } from "vue";

function patchReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultStruct(StructType.READ_OPTIONS),
    ...options,
  };
}

export function getScopeKey(scope: GraphScope): string {
  return JSON.stringify(scope);
}

/**
 * A connection to a subgraph for an overlapping set of read operations.
 */
export interface GraphConnection {
  readonly readTypes: ReadType[];
  readonly scope: GraphScope;
  readonly graph: ReadNodeGraph;
  readonly access: AccessArbiter;
  readonly tx: TransactionBuffer; // may be shared across connections
  readonly options: ReadOptionsData;
  readonly isLive: boolean; // whether this connection is watched
  readonly isActive: boolean;
}

export class LocalGraphConnection implements GraphConnection {
  readonly readTypes: ReadType[];
  readonly scope: GraphScope;
  readonly graph: ReadNodeGraph;
  readonly access: AccessArbiter;
  readonly tx: TransactionBuffer;
  readonly options: ReadOptionsData;

  constructor(
    readTypes: ReadType[],
    scope: GraphScope,
    graph: ReadNodeGraph,
    tx: TransactionBuffer,
    options: ReadOptionsData,
  ) {
    this.readTypes = readTypes;
    this.scope = scope;
    this.graph = graph;
    this.access = accessAsOwner();
    this.tx = tx;
    this.options = options;
  }

  isLive = true; // always considered live
  isActive = true; // always considered active
}

export class ProxyGraphConnection implements GraphConnection {
  readonly connection: ShallowRef<GraphConnection | null>;

  constructor(connection: MaybeRef<GraphConnection | null>) {
    this.connection = !isRef(connection) ? shallowRef(connection) : (connection as ShallowRef<GraphConnection | null>);
  }

  get activeConnection(): GraphConnection {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value;
  }

  get readTypes(): ReadType[] {
    return this.activeConnection.readTypes;
  }

  get scope(): GraphScope {
    return this.activeConnection.scope;
  }

  get graph(): ReadNodeGraph {
    return this.activeConnection.graph;
  }

  get access(): AccessArbiter {
    return this.activeConnection.access;
  }

  get tx(): TransactionBuffer {
    return this.activeConnection.tx;
  }

  get options(): ReadOptionsData {
    return this.activeConnection.options;
  }

  get isLive(): boolean {
    return this.activeConnection.isLive;
  }

  get isActive(): boolean {
    return this.activeConnection.isActive;
  }
}

const graphConnections: Ref<GraphConnection[]> = shallowRef([]);

export function addGraphConnection(connection: GraphConnection): void {
  graphConnections.value = [...graphConnections.value, connection];
}

/**
 * Gets the currently loaded graph for the given scope.
 */
export function useLoadedGraph(node: MaybeRef<NodeReferenceData>): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
} {
  const nodeRef = toRef(node) as Ref<NodeReferenceData>;

  throw new Error("not yet implemented");
}

/**
 * Gets the given nodes from the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that edits for the given nodes are watched.
 */
export function useGetNodes<T extends NodeType>(
  request: MaybeRef<{
    roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
    options?: Partial<ReadOptionsData>;
    live?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
} {
  const graphProxy = new ProxyNodeGraph(null);
  const connectionProxy = new ProxyGraphConnection(null);

  throw new Error("not yet implemented");
}

export type PageInfo = { cursors: string[]; startCursor: string; size: number; total?: number };

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function useSearchNodes<T extends NodeType>(
  request: MaybeRef<{
    nodeType: T;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    first?: number;
    skip?: number;
    after?: string | null;
    options?: Partial<ReadOptionsData>;
    count?: boolean;
    live?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
  page: Ref<PageInfo>;
} {
  throw new Error("not yet implemented");
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * TODO :Feature: live aggregation
 */
export function useAggregateNodes(
  aggregate: MaybeRef<{
    nodeType: NodeType;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    aggregation: ExpressionData;
    enabled?: boolean;
    live?: boolean;
  }>,
): { aggregation: SubRef<AggregationData> } {
  throw new Error("not yet implemented");
}
