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
  Timestamp,
} from "@/proto/wire";
import { makeDefaultProto, unwrapSomeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { accessAsOwner, type AccessArbiter } from "@/system/access";
import { NodeGraph, ProxyNodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { LOCAL_BENCH_ID, LOCAL_PACKAGE_ID } from "@/system/client";
import {
  ImmediateTransactionBuffer,
  SwapTransactionBuffer,
  editGraph,
  type Transaction,
  type TransactionBuffer,
  canonicalizeEdits,
} from "@/system/transaction";
import { log } from "@/utils/log";
import { toValueRef, type SubRef, pretendReadonly } from "@/utils/ref";
import { tryOnBeforeUnmount } from "@vueuse/core";
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
 * A connection to a subgraph.
 */
export type GraphConnection = {
  id: number;
  name: string;
  kind: GraphConnectionKind;
  scope: GraphScope;
  graph: ReadNodeGraph;
  access: AccessArbiter;
  options: ReadOptionsData;
  mainTx: Transaction;
  sideTx: Transaction;
  isLive: boolean;
  isFetching: boolean;
  isUsed: boolean;
  referenceCount: number;
};

export abstract class GraphConnectionBase implements GraphConnection {
  readonly id: number;
  readonly name: string;
  readonly kind: GraphConnectionKind;
  readonly scope: GraphScope;
  readonly graph: ReadNodeGraph;
  readonly access: AccessArbiter;
  readonly options: ReadOptionsData;
  referenceCount: number;

  abstract readonly mainTx: Transaction;
  abstract readonly sideTx: Transaction;
  abstract readonly isLive: boolean;
  abstract readonly isFetching: boolean;

  constructor(
    id: number,
    name: string,
    kind: GraphConnectionKind,
    scope: GraphScope,
    graph: ReadNodeGraph,
    access: AccessArbiter,
    options: ReadOptionsData,
  ) {
    this.id = id;
    this.name = name;
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
    id: number,
    name: string,
    kind: GraphConnectionKind,
    scope: GraphScope,
    graph: ReadNodeGraph & WriteNodeGraph,
    options: ReadOptionsData,
  ) {
    super(id, name, kind, scope, graph, accessAsOwner(), options);
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
  isFetching = false; // never fetching

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
  readonly _isFetching: Ref<boolean>;
  readonly mainTxBuffer: TransactionBuffer;
  readonly sideTxBuffer: TransactionBuffer;

  constructor(
    id: number,
    name: string,
    kind: GraphConnectionKind,
    scope: GraphScope,
    graph: ReadNodeGraph,
    access: AccessArbiter,
    client: IGraphIOClient,
    options: ReadOptionsData,
    isLive: boolean,
  ) {
    super(id, name, kind, scope, graph, access, options);
    this.client = client;
    this.isLive = isLive;
    this._isFetching = shallowRef(false);
    this.mainTxBuffer = new SwapTransactionBuffer(graph.scope, client);
    this.sideTxBuffer = new SwapTransactionBuffer(graph.scope, client);
  }

  get isFetching(): boolean {
    return this._isFetching.value;
  }

  set isFetching(value: boolean) {
    this._isFetching.value = value;
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

  get id(): number {
    return this.activeConnection.id;
  }

  get name(): string {
    return this.activeConnection.name;
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

  get isFetching(): boolean {
    return this.activeConnection.isFetching;
  }

  get isUsed(): boolean {
    return this.activeConnection.isUsed;
  }
}

// define local space graph here because we use it immediately
export const spaceGraphLocal = new NodeGraph({ scope: { benchId: LOCAL_BENCH_ID, packageId: LOCAL_PACKAGE_ID } });

let connectionId = 0;
function newConnectionId(): number {
  return connectionId++;
}

const _graphConnections: Ref<GraphConnection[]> = shallowRef([
  // add local graph
  new LocalGraphConnection(
    newConnectionId(),
    "local.space",
    "get",
    spaceGraphLocal.scope,
    spaceGraphLocal,
    makeReadOptions({ descendantTypes: [NodeType.VIEW] }),
  ),
]);
export const graphConnections = pretendReadonly(_graphConnections);

export function addGraphConnection(connection: GraphConnection): void {
  _graphConnections.value = [..._graphConnections.value, connection];
}

export type PageInfo = { cursors: string[]; startCursor: string; size: number; total?: number };

type NodeRequestParams = {
  /** Identifies the request/connection for logging. Client side only. */
  name: string;
  live?: boolean;
  /** Whether  */
  enabled?: boolean;
};

type GetNodesParams<T extends NodeType> = NodeRequestParams & {
  roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
  options?: Partial<ReadOptionsData>;
};

type SearchNodesParams<T extends NodeType> = NodeRequestParams & {
  nodeType: T;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  first?: number;
  skip?: number;
  after?: string | null;
  options?: Partial<ReadOptionsData>;
  count?: boolean;
};

type AggregateNodesParams = NodeRequestParams & {
  nodeType: NodeType;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  aggregation: ExpressionData;
};

/**
 * Finds an existing connection to the relevant subgraph.
 */
export function findGetConnection<T extends NodeType>(params: Omit<GetNodesParams<T>, "name">): GraphConnection | null {
  const connection = _graphConnections.value.find((c) => {
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
export async function connectGet<T extends NodeType>(
  params: Omit<GetNodesParams<T>, "enabled">,
): Promise<{ connection: GraphConnection }> {
  const existingConnection = findGetConnection(params);
  if (existingConnection != null) {
    existingConnection.referenceCount += 1;
    return { connection: existingConnection };
  }

  log.info(`graph.get.${params.name}`, params);
  const benchId = params.roots[0]!.benchId;
  const client = benchId == null ? supervisor : await getHostClient({ id: benchId });
  const graph = new NodeGraph({ scope: { benchId } });
  const access = accessAsOwner(); // TODO :Broken: access control
  const options = makeReadOptions(params.options ?? {});
  const connection = new RemoteGraphConnection(
    newConnectionId(),
    params.name,
    "get",
    graph.scope,
    graph,
    access,
    client,
    options,
    params.live ?? false,
  );
  addGraphConnection(connection);

  // TODO :Robusness: retry/resume watching connection on watch failure
  
  // fetch nodes
  let epoch: bigint;
  try {
    connection.isFetching = true;
    const {
      response: { epoch: fetchedEpoch, nodes },
    } = await client.getNodes({ scope: graph.scope, roots: params.roots, options } as GetNodesRequest);
    epoch = fetchedEpoch;
    graph.extend(...nodes.map(unwrapSomeNode));
  } finally {
    connection.isFetching = false;
  }

  // watch edits if live
  if (params.live) {
    const allNodeTypes = [...params.roots.map((r) => r.type), ...options.ancestorTypes, ...options.descendantTypes];
    const editStream = client.watchEdits({
      scope: graph.scope,
      sinceEpoch: epoch,
      nodeTypes: allNodeTypes,
      filters: [],
    });
    editStream.responses.onNext((tx) => {
      if (tx != null) {
        canonicalizeEdits(Timestamp.now(), tx.edits);
        editGraph(graph, tx.edits);
      }
    });
  }

  connection.referenceCount += 1;
  tryOnBeforeUnmount(() => connection.referenceCount--);
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
      if (!paramsRef.value.enabled) {
        const oldConnection = connectionProxy.connection.value;
        if (oldConnection != null) {
          // TODO :Broken: close connection
        }
        return;
      }

      const { connection } = await connectGet(paramsRef.value);
      if (connectionProxy.connection.value != null) {
        connectionProxy.connection.value.referenceCount -= 1;
      }
      connectionProxy.connection.value = connection;
      graphProxy.graph = connection.graph;
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
 * NOTE: for performance the graph/connection proxies are 'lazy' (batched per tick as regular refs).
 *  That means changing 'node' will change connection/graph only on the next tick.
 */
export function useActiveConnection(node: MaybeRef<NodeReferenceData | TypedNodeReferenceData<any> | null>): {
  graph: ReadNodeGraph;
  connection: GraphConnection;
} {
  const nodeRef = toRef(node) as Ref<NodeReferenceData>;
  const graphProxy = new ProxyNodeGraph(null);
  const connectionProxy = new ProxyGraphConnection(null);

  // route to the appropriate graph connection
  watch(
    toValueRef(nodeRef),
    () => {
      if (nodeRef.value == null) {
        connectionProxy.connection.value = null;
        graphProxy.graph = null;
      } else {
        const connection = findGetConnection({ roots: [nodeRef.value] });
        if (connection !== connectionProxy.connection.value) {
          if (connectionProxy.connection.value) connectionProxy.connection.value.referenceCount -= 1;
          connectionProxy.connection.value = connection;
          if (connection) connection.referenceCount += 1;
        }
        if (connection?.graph !== graphProxy.graph) graphProxy.graph = connection?.graph ?? null;
      }
    },
    { immediate: true },
  );

  return { graph: markRaw(graphProxy), connection: markRaw(connectionProxy) };
}
