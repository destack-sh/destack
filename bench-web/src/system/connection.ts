import { getHostClient, supervisor } from "@/proto/services";
import {
  AggregationData,
  BenchType,
  ExpressionData,
  GetNodesRequest,
  NodeType,
  type GraphScope,
  type IGraphIOClient,
  type NodeReferenceData,
  type NodeTypeMapping,
  type ReadOptionsData,
} from "@/proto/wire";
import { makeDefaultProto, unwrapSomeNode } from "@/proto/wiring";
import { accessAsOwner, type AccessArbiter } from "@/system/access";
import { NodeGraph, ProxyNodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { LOCAL_BENCH_ID, LOCAL_PACKAGE_ID } from "@/system/local";
import {
  ImmediateTransactionBuffer,
  SwapTransactionBuffer,
  editGraph,
  type Transaction,
  type TransactionBuffer,
} from "@/system/transaction";
import { log } from "@/utils/log";
import { onUnmountedIfComponent, toValueRef, type SubRef } from "@/utils/ref";
import { computed, isRef, shallowRef, toRef, watch, type MaybeRef, type Ref, type ShallowRef, markRaw } from "vue";

export function makeReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultProto(BenchType.READ_OPTIONS),
    ...options,
  };
}

export function getScopeKey(scope: GraphScope): string {
  return JSON.stringify(scope);
}

export type GraphConnectionKind = "get" | "search" | "aggregate";

/**
 * A connection to a subgraph for an overlapping set of read operations.
 */
export type GraphConnection = {
  kind: GraphConnectionKind;
  scope: GraphScope;
  graph: ReadNodeGraph;
  access: AccessArbiter;
  options: ReadOptionsData;
  mainTx: Transaction;
  sideTx: Transaction;
  isLive: boolean;
  isUsed: boolean;
  referenceCount: number;
};

export abstract class GraphConnectionBase implements GraphConnection {
  readonly kind: GraphConnectionKind;
  readonly scope: GraphScope;
  readonly graph: ReadNodeGraph;
  readonly access: AccessArbiter;
  readonly options: ReadOptionsData;
  referenceCount: number;

  abstract readonly mainTx: Transaction;
  abstract readonly sideTx: Transaction;
  abstract readonly isLive: boolean;

  constructor(
    kind: GraphConnectionKind,
    scope: GraphScope,
    graph: ReadNodeGraph,
    access: AccessArbiter,
    options: ReadOptionsData,
  ) {
    this.kind = kind;
    this.scope = scope;
    this.graph = graph;
    this.access = access;
    this.options = options;
    this.referenceCount = 0;
  }

  get isUsed(): boolean {
    return this.referenceCount > 0;
  }
}

/**
 * A connection to a graph we have in memory on the client.
 */
export class LocalGraphConnection extends GraphConnectionBase {
  readonly mainTxBuffer: TransactionBuffer;
  readonly sideTxBuffer: TransactionBuffer;

  constructor(
    kind: GraphConnectionKind,
    scope: GraphScope,
    graph: ReadNodeGraph & WriteNodeGraph,
    options: ReadOptionsData,
  ) {
    super(kind, scope, graph, accessAsOwner(), options);
    this.mainTxBuffer = new ImmediateTransactionBuffer(graph.scope, graph);
    this.sideTxBuffer = new ImmediateTransactionBuffer(graph.scope, graph);
  }

  get mainTx(): Transaction {
    return this.mainTxBuffer.tx;
  }

  get sideTx(): Transaction {
    return this.sideTxBuffer.tx;
  }

  isLive = true; // always considered live

  get isUsed(): boolean {
    return true; // always active
  }
}

/**
 * A remote connection to a graph.
 */
export class RemoteGraphConnection extends GraphConnectionBase {
  readonly client: IGraphIOClient;
  readonly isLive: boolean;
  readonly mainTxBuffer: TransactionBuffer;
  readonly sideTxBuffer: TransactionBuffer;

  constructor(
    kind: GraphConnectionKind,
    scope: GraphScope,
    graph: ReadNodeGraph,
    access: AccessArbiter,
    client: IGraphIOClient,
    options: ReadOptionsData,
    isLive: boolean,
  ) {
    super(kind, scope, graph, access, options);
    this.client = client;
    this.isLive = isLive;
    this.mainTxBuffer = new SwapTransactionBuffer(graph.scope, client);
    this.sideTxBuffer = new SwapTransactionBuffer(graph.scope, client);
  }

  get mainTx(): Transaction {
    return this.mainTxBuffer.tx;
  }

  get sideTx(): Transaction {
    return this.sideTxBuffer.tx;
  }
}

/**
 * A proxy to an underlying connection so we can swap it out with a stable reference.
 */
export class ProxyGraphConnection implements GraphConnection {
  readonly connection: ShallowRef<GraphConnection | null>;

  constructor(connection: MaybeRef<GraphConnection | null>) {
    this.connection = !isRef(connection) ? shallowRef(connection) : (connection as ShallowRef<GraphConnection | null>);
  }

  get activeConnection(): GraphConnection {
    if (this.connection.value == null) throw new Error("no current connection");
    return this.connection.value;
  }

  get kind(): GraphConnectionKind {
    return this.activeConnection.kind;
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

  get mainTx(): Transaction {
    return this.activeConnection.mainTx;
  }

  get sideTx(): Transaction {
    return this.activeConnection.sideTx;
  }

  get options(): ReadOptionsData {
    return this.activeConnection.options;
  }

  get referenceCount(): number {
    return this.activeConnection.referenceCount;
  }

  set referenceCount(value: number) {
    this.activeConnection.referenceCount = value;
  }

  get isLive(): boolean {
    return this.activeConnection.isLive;
  }

  get isUsed(): boolean {
    return this.activeConnection.isUsed;
  }
}

// define local space graph here because we use it immediately
export const spaceGraphLocal = new NodeGraph({ scope: { benchId: LOCAL_BENCH_ID, packageId: LOCAL_PACKAGE_ID } });

const graphConnections: Ref<GraphConnection[]> = shallowRef([
  // add local graph
  new LocalGraphConnection(
    "get",
    spaceGraphLocal.scope,
    spaceGraphLocal,
    makeReadOptions({ descendantTypes: [NodeType.VIEW] }),
  ),
]);

export function addGraphConnection(connection: GraphConnection): void {
  graphConnections.value = [...graphConnections.value, connection];
}

export type PageInfo = { cursors: string[]; startCursor: string; size: number; total?: number };

type GetNodesParams<T extends NodeType> = {
  roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
  options?: Partial<ReadOptionsData>;
  live?: boolean;
  enabled?: boolean;
};

type SearchNodesParams<T extends NodeType> = {
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
};

type AggregateNodesParams = {
  nodeType: NodeType;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  aggregation: ExpressionData;
  enabled?: boolean;
  live?: boolean;
};

/**
 * Finds an existing connection to the relevant subgraph.
 */
export function findGetConnection<T extends NodeType>(params: GetNodesParams<T>): GraphConnection | null {
  const connection = graphConnections.value.find((c) => {
    if (c.kind != "get") return false;
    if (params?.live && !c.isLive) return false;
    // scope included?
    if (params.roots.some((r) => r.benchId != c.scope.benchId)) return false;
    // options included?
    if (params.options?.ancestorTypes?.some((t) => !c.options.ancestorTypes?.includes(t))) return false;
    if (params.options?.descendantTypes?.some((t) => !c.options.descendantTypes?.includes(t))) return false;
    return true;
  });
  return connection ?? null;
}

/**
 * Gets an existing or creates a new connection to the relevant subgraph.
 */
export async function acquireGetConnection<T extends NodeType>(
  params: Omit<GetNodesParams<T>, "enabled">,
): Promise<{ connection: GraphConnection }> {
  const existingConnection = findGetConnection(params);
  if (existingConnection != null) {
    existingConnection.referenceCount += 1;
    return { connection: existingConnection };
  }

  log.info("acquireGetConnection", params);
  const benchId = params.roots[0]!.benchId;
  const client = benchId == null ? supervisor : await getHostClient({ id: benchId });
  const graph = new NodeGraph({ scope: { benchId } });
  const access = accessAsOwner(); // TODO :Broken: access control
  const options = makeReadOptions(params.options ?? {});
  const connection = new RemoteGraphConnection(
    "get",
    graph.scope,
    graph,
    access,
    client,
    options,
    params.live ?? false,
  );
  addGraphConnection(connection);
  const {
    response: { epoch, nodes },
  } = await client.getNodes({ scope: graph.scope, roots: params.roots, options } as GetNodesRequest);
  graph.extend(...nodes.map(unwrapSomeNode));

  if (params.live) {
    const allNodeTypes = [...params.roots.map((r) => r.type), ...options.ancestorTypes, ...options.descendantTypes];
    const editStream = client.watchEdits({
      scope: graph.scope,
      sinceEpoch: epoch,
      nodeTypes: allNodeTypes,
      filters: [],
    });
    editStream.responses.onNext((tx) => {
      if (tx) {
        editGraph(graph, tx.edits);
      }
    });
  }

  onUnmountedIfComponent(() => connection.referenceCount--);
  return { connection };
}

/**
 * Gets the given nodes from the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that edits for the given nodes are watched.
 */
export function useGetNodes<T extends NodeType>(
  params: MaybeRef<GetNodesParams<T>>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
} {
  const paramsRef = toRef(params) as Ref<GetNodesParams<T>>;
  const graphProxy = new ProxyNodeGraph(null);
  const connectionProxy = new ProxyGraphConnection(null);

  // route to the relevant graph connection
  watch(
    toValueRef(paramsRef),
    async () => {
      if (!paramsRef.value.enabled) return;
      const { connection } = await acquireGetConnection(paramsRef.value);
      if (connectionProxy.connection.value != null) {
        connectionProxy.connection.value.referenceCount -= 1;
      }
      connectionProxy.connection.value = connection;
      graphProxy.graph.value = connection.graph;
    },
    { immediate: true },
  );

  const roots = graphProxy.getManyRef(computed(() => paramsRef.value.roots));
  return { graph: markRaw(graphProxy), connection: markRaw(connectionProxy), roots };
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function useSearchNodes<T extends NodeType>(
  params: MaybeRef<SearchNodesParams<T>>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
  roots: SubRef<NodeTypeMapping[T][]>;
  page: Ref<PageInfo>;
} {
  const paramsRef = toRef(params) as Ref<SearchNodesParams<T>>;
  const graphProxy = new ProxyNodeGraph(null);
  const connectionProxy = new ProxyGraphConnection(null);
  throw new Error("not yet implemented");
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * TODO :Feature: live aggregation
 */
export function useAggregateNodes(params: MaybeRef<AggregateNodesParams>): {
  aggregation: SubRef<AggregationData>;
} {
  const paramsRef = toRef(params) as Ref<AggregateNodesParams>;
  throw new Error("not yet implemented");
}

/**
 * Gets the currently loaded graph for the given scope. Does not acquire any new connections.
 */
export function useLoadedGraph(node: MaybeRef<NodeReferenceData>): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
} {
  const nodeRef = toRef(node) as Ref<NodeReferenceData>;
  const graphProxy = new ProxyNodeGraph(null);
  const connectionProxy = new ProxyGraphConnection(null);

  // route to the relevant graph connection
  watch(
    toValueRef(nodeRef),
    async () => {
      const connection = findGetConnection({ roots: [nodeRef.value] });
      if (connection != null) {
        connectionProxy.connection.value = connection;
        graphProxy.graph.value = connection.graph;
      } else {
        connectionProxy.connection.value = null;
        graphProxy.graph.value = null;
      }
    },
    { immediate: true },
  );

  return { graph: markRaw(graphProxy), connection: markRaw(connectionProxy) };
}
