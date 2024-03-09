import {
  StructType,
  type GraphScope,
  type IGraphIOClient,
  type NodeReferenceData,
  type NodeType,
  type ReadOptionsData,
  type NodeTypeMapping,
  ReadType,
} from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import { accessAsOwner, type AccessQuery } from "@/system/access";
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
  readonly scope: GraphScope;
  readonly graph: ReadNodeGraph;
  readonly access: AccessQuery;
  readonly tx: TransactionBuffer; // may be shared across connections
  readonly options: ReadOptionsData;
  readonly live: boolean; // whether this connection is watched

  /** Whether this connection supports the given read */
  supports(read: ReadType, scope: GraphScope, root: NodeReferenceData, options: ReadOptionsData): boolean;
}

export class LocalGraphConnection implements GraphConnection {
  readonly scope: GraphScope;
  readonly graph: ReadNodeGraph;
  readonly access: AccessQuery;
  readonly tx: TransactionBuffer;
  readonly options: ReadOptionsData;

  constructor(scope: GraphScope, graph: ReadNodeGraph, tx: TransactionBuffer, options: ReadOptionsData) {
    this.scope = scope;
    this.graph = graph;
    this.access = accessAsOwner();
    this.tx = tx;
    this.options = options;
  }
}

export class RemoteGraphConnection implements GraphConnection {
  readonly client: IGraphIOClient;
}

export class ProxyGraphConnection implements GraphConnection {
  readonly connection: ShallowRef<GraphConnection | null>;

  constructor(connection: MaybeRef<GraphConnection | null>) {
    this.connection = !isRef(connection) ? shallowRef(connection) : (connection as ShallowRef<GraphConnection | null>);
  }

  get scope(): GraphScope {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.scope;
  }

  get graph(): ReadNodeGraph {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.graph;
  }

  get access(): AccessQuery {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.access;
  }

  get tx(): TransactionBuffer {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.tx;
  }

  get options(): ReadOptionsData {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.options;
  }

  get live(): boolean {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.live;
  }

  supports(read: ReadType, scope: GraphScope, root: NodeReferenceData, options: ReadOptionsData): boolean {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value.supports(read, scope, root, options);
  }
}

const graphConnections: Ref<GraphConnection[]> = shallowRef([]);

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
 * If watch, will also ensure that edits for the given nodes are watched.
 */
export function useGetNodes<T extends NodeType>(
  request: MaybeRef<{
    roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
    options?: Partial<ReadOptionsData>;
    watch?: boolean;
    enabled?: boolean;
  }>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
} {
  const graph = new ProxyNodeGraph(null);
  const connection = new ProxyGraphConnection(null);

  throw new Error("not yet implemented");
}

export type PageInfo = { cursors: string[]; startCursor: string; size: number; total?: number };

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * If watch, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
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
    watch?: boolean;
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
 * TODO :Feature: watch aggregation
 */
export function useAggregateNodes(
  aggregate: MaybeRef<{
    nodeType: NodeType;
    bases?: NodeReferenceData[];
    filter?: ExpressionData;
    sort?: ExpressionData[];
    aggregation: ExpressionData;
    enabled?: boolean;
  }>,
): { aggregation: SubRef<AggregationData> } {
  throw new Error("not yet implemented");
}
