import { getGraphClient } from "@/proto/services";
import {
  AggregationData,
  BenchType,
  ExpressionData,
  NodeType,
  Timestamp,
  WatchEditsResponse,
  type GraphScope,
  type NodeReferenceData,
  type NodeTypeMapping,
  type ReadOptionsData,
} from "@/proto/wire";
import { makeDefaultProto, unwrapSomeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { AccessProxy, accessAsOwner, accessFromMatrix, type AccessArbiter } from "@/system/access";
import { LOCAL_BENCH_ID, LOCAL_PACKAGE_ID, LOCAL_SPACE_PTR } from "@/system/client";
import { NodeGraph, ProxyNodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import {
  ImmediateTransactionBuffer,
  canonicalizeEdits,
  editGraph,
  getTransactionBuffer,
  type Transaction,
  type TransactionBuffer,
} from "@/system/transaction";
import { log } from "@/utils/log";
import { deepValueEquals, pretendReadonly, proxyWrap, toValueRef, type SubRef } from "@/utils/ref";
import { computed, isRef, markRaw, shallowRef, toRef, watch, type MaybeRef, type Ref, type ShallowRef } from "vue";

export function makeReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultProto(BenchType.READ_OPTIONS),
    ...options,
  };
}

export function getScopeKey(scope: GraphScope): string {
  return JSON.stringify(scope);
}

//
// Connection typing.
// NOTE: we re-type the graph connection params here to relax some constraints for convenience
//

export type PageInfo = { cursors: string[]; startCursor: string; size: number; total?: number };

type ConnectOptions = {};

type ConnectionMetadata = {
  /** Identifies the request/connection for logging. Client side only. */
  id: number;
  name: string;
  live: boolean;
  enabled: boolean;
  options: ConnectOptions;
};

type GetConnectionParams<T extends NodeType> = {
  roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
  scope?: Partial<GraphScope>;
  options?: Partial<ReadOptionsData>;
};

type GetConnectionResult<T extends NodeType> = {
  access: AccessArbiter;
  graph: ReadNodeGraph;
  roots: SubRef<NodeTypeMapping[T][]>;
};

type SearchConnectionParams<T extends NodeType> = {
  nodeType: T;
  scope?: Partial<GraphScope>;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  first?: number;
  skip?: number;
  after?: string;
  options?: Partial<ReadOptionsData>;
  count?: boolean;
};

type SearchConnectionResult<T extends NodeType> = {
  access: AccessArbiter;
  graph: ReadNodeGraph;
  roots: SubRef<NodeTypeMapping[T][]>;
  page: PageInfo;
};

type AggregateConnectionParams = {
  nodeType: NodeType;
  scope?: Partial<GraphScope>;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  aggregation: ExpressionData;
};

type AggregateConnectionResult = {
  aggregation: SubRef<AggregationData>;
};

export type GraphConnectionKind = "get" | "search" | "aggregate";
interface ConnectionParamsMapping<T extends NodeType> extends Record<GraphConnectionKind, any> {
  get: GetConnectionParams<T>;
  search: SearchConnectionParams<T>;
  aggregate: AggregateConnectionParams;
}
interface ConnectionResultMapping<T extends NodeType> extends Record<GraphConnectionKind, any> {
  get: GetConnectionResult<T>;
  search: SearchConnectionResult<T>;
  aggregate: AggregateConnectionResult;
}

function getScopeFromParams<T extends NodeType>(params: ConnectionParamsMapping<T>[GraphConnectionKind]): GraphScope {
  if (params.scope != null) return params.scope;
  if ("roots" in params && params.roots.length > 0) return { benchId: params.roots[0].id };
  if ("bases" in params && (params.bases?.length ?? 0) > 0) return { benchId: params.bases![0].id };

  throw new Error(`cannot determine scope from params: ${JSON.stringify(params)}`);
}

function applyRemoteEdits(rep: WatchEditsResponse, graph: ReadNodeGraph & WriteNodeGraph): void {
  canonicalizeEdits(Timestamp.now(), rep.edits);
  editGraph(graph, rep.edits);
}

//
// Connections
//

/** A connection to a subgraph. */
export abstract class GraphConnection<K extends GraphConnectionKind, T extends NodeType> {
  abstract readonly kind: K;

  /** Immutable-after-construction metadata about this connection. */
  readonly meta: ConnectionMetadata;
  readonly params: Ref<ConnectionParamsMapping<T>[K]>;
  readonly result: ShallowRef<ConnectionResultMapping<T>[K] | null> = shallowRef(null);
  readonly txBuffer: TransactionBuffer;

  readonly isConnected: Ref<boolean> = shallowRef(false);
  readonly isFetching: Ref<boolean> = shallowRef(false);
  abortController: AbortController | null = null; // for active fetch
  referenceCount: number = 0;

  constructor(meta: ConnectionMetadata, params: MaybeRef<ConnectionParamsMapping<T>[K]>, txBuffer?: TransactionBuffer) {
    this.meta = meta;
    this.params = isRef(params) ? params : shallowRef(params);
    this.txBuffer = txBuffer ?? getTransactionBuffer(getScopeFromParams(this.params.value));
  }

  get id(): number {
    return this.meta.id;
  }

  get name(): string {
    return this.meta.name;
  }

  get isLive(): boolean {
    return this.meta.live;
  }

  get isEnabled(): boolean {
    return this.meta.enabled;
  }

  get tx(): Transaction {
    return this.txBuffer.tx;
  }

  /** Maintain this connection until the end of time (or until aborted). */
  async connect(options?: Partial<ConnectOptions>, abort?: AbortController): Promise<void> {
    if (this.isConnected.value) throw new Error("already connected");
    options = { ...this.meta.options, ...options };
    // nocheckin
  }

  /**
   * Fetch the results of this connection for the given params once.
   * If live, also updates the results from the source (until aborted).
   * */
  async fetch(params: ConnectionParamsMapping<T>[K] & { scope?: GraphScope }): Promise<void> {
    if (this.isFetching.value) this.abortController?.abort();
    log.trace("connection." + this.kind, this.meta.name, params);
    this.isFetching.value = true;
    this.abortController = new AbortController();
    try {
      const scope = params.scope ?? getScopeFromParams(params);
      const result = await this.doFetch(scope, params, this.abortController.signal);
      this.result.value = result;
      this.abortController = null;
    } finally {
      if (this.abortController) this.abortController.abort(); // cleanup
      this.isFetching.value = false;
    }
  }

  /** Actually fetch. */
  protected abstract doFetch(
    scope: GraphScope,
    params: ConnectionParamsMapping<T>[K],
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<ConnectionResultMapping<T>[K]>;

  /** Whether this connection is a superset of the given connection */
  supports(params: ConnectionParamsMapping<T>[K]): boolean {
    if (this.kind == "get") {
      // TODO :Broken: Connection.supports is incorrect sometimes (assumes )
      const thisGet = this.params.value as GetConnectionParams<T>;
      const otherGet = params as GetConnectionParams<T>;
      // scope included?
      if (getScopeFromParams(otherGet).benchId != getScopeFromParams(thisGet).benchId) return false;
      // options included?
      if (otherGet.options?.ancestorTypes?.some((t) => !thisGet.options?.ancestorTypes?.includes(t))) return false;
      if (otherGet.options?.descendantTypes?.some((t) => !thisGet.options?.descendantTypes?.includes(t))) return false;
      return true;
    } else if (this.kind == "search") {
      return deepValueEquals(this.params.value, params);
    } else if (this.kind == "aggregate") {
      return deepValueEquals(this.params.value, params);
    } else {
      throw new Error(`unsupported connection kind: ${this.kind}`);
    }
  }
}

export class RemoteGetConnection<T extends NodeType> extends GraphConnection<"get", T> {
  readonly kind = "get";

  protected async doFetch(
    scope: GraphScope,
    params: GetConnectionParams<T>,
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<GetConnectionResult<T>> {
    const client = await getGraphClient(scope);
    const graph = new NodeGraph({ scope });
    const options = makeReadOptions(params.options ?? {});

    // fetch nodes
    const {
      response: { epoch, access: accessMatrix, nodes },
    } = await client.getNodes({ scope: graph.scope, roots: params.roots, options }, { abort });
    const access = accessFromMatrix(accessMatrix!);
    graph.extend(...nodes.map(unwrapSomeNode));

    // watch edits if live
    if (this.isLive) {
      const allNodeTypes = [...params.roots.map((r) => r.type), ...options.ancestorTypes, ...options.descendantTypes];
      const editStream = client.watchEdits(
        {
          scope: graph.scope,
          sinceEpoch: epoch,
          nodeTypes: allNodeTypes,
          filters: [],
        },
        { abort },
      );
      editStream.responses.onNext((rep) => rep != null && applyRemoteEdits(rep, graph));
      editStream.responses.onError(onError);
    }

    return { graph, access, roots: graph.getManyRef(params.roots) };
  }
}

export class RemoteSearchConnection<T extends NodeType> extends GraphConnection<"search", T> {
  readonly kind = "search";

  protected async doFetch(
    scope: GraphScope,
    params: SearchConnectionParams<T>,
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<SearchConnectionResult<T>> {
    const client = await getGraphClient(scope);
    const graph = new NodeGraph({ scope });
    const options = makeReadOptions(params.options ?? {});

    // fetch nodes
    const {
      response: { epoch, access: accessMatrix, nodes, roots, cursors, startCursor, total },
    } = await client.searchNodes({
      ...params,
      nodeType: params.nodeType,
      bases: params.bases ?? [],
      scope: graph.scope,
      sort: params.sort ?? [],
      options,
    });
    const access = accessFromMatrix(accessMatrix!);
    const page = { cursors, startCursor, size: roots.length, total };
    graph.extend(...nodes.map(unwrapSomeNode));

    // watch edits if live
    if (this.isLive) {
      const allNodeTypes = [params.nodeType, ...options.ancestorTypes, ...options.descendantTypes];
      const editStream = client.watchEdits(
        {
          scope: graph.scope,
          sinceEpoch: epoch,
          nodeTypes: allNodeTypes,
          filters: [],
        },
        { abort },
      );
      editStream.responses.onNext((rep) => rep != null && applyRemoteEdits(rep, graph));
      editStream.responses.onError(onError);
    }

    return { graph, access, roots: graph.getManyRef(roots as TypedNodeReferenceData<T>[]), page };
  }
}

export class LocalGetConnection<T extends NodeType> extends GraphConnection<"get", T> {
  readonly kind = "get";

  readonly graph: ReadNodeGraph;

  constructor(
    meta: ConnectionMetadata,
    params: MaybeRef<GetConnectionParams<T>>,
    graph: WriteNodeGraph & ReadNodeGraph,
  ) {
    super(meta, params, new ImmediateTransactionBuffer(graph.scope, graph));
    this.graph = graph;
  }

  protected async doFetch(scope: GraphScope, params: GetConnectionParams<T>): Promise<GetConnectionResult<T>> {
    return { graph: this.graph, access: accessAsOwner(), roots: this.graph.getManyRef(params.roots) };
  }
}

//
// Maintaining and routing connections
//

// define local space graph here because we use it immediately
export const spaceGraphLocal = new NodeGraph({ scope: { benchId: LOCAL_BENCH_ID, packageId: LOCAL_PACKAGE_ID } });

let connectionId = 0;
function newConnectionId(): number {
  return connectionId++;
}
const _graphConnections: Ref<GraphConnection<any, any>[]> = shallowRef([
  // add local graph
  new LocalGetConnection(
    {
      id: newConnectionId(),
      name: "local.space",
      live: true,
      enabled: true,
      options: {},
    },
    {
      roots: [LOCAL_SPACE_PTR],
      options: makeReadOptions({ descendantTypes: [NodeType.VIEW] }),
    },
    spaceGraphLocal,
  ),
]);
export const graphConnections = pretendReadonly(_graphConnections);

export function addGraphConnection(connection: GraphConnection<any, any>): void {
  _graphConnections.value = [..._graphConnections.value, connection];
}

export function findConnection<K extends GraphConnectionKind, T extends NodeType>(
  params: ConnectionParamsMapping<T>[K],
): GraphConnection<K, T> | null {
  return _graphConnections.value.find((c) => c.supports(params)) ?? null;
}

/**
 * Gets the given nodes from the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that edits for the given nodes are watched.
 */
export function useGetNodes<T extends NodeType>(
  params: MaybeRef<GetConnectionParams<T>>,
): GetConnectionResult<T> & { connection: GraphConnection<"get", T> } {
  const paramsRef = toRef(params) as Ref<GetConnectionParams<T>>;
  const graph = new ProxyNodeGraph(null);
  const connection: ShallowRef<GraphConnection<"get", T> | null> = shallowRef(null);
  const access = new AccessProxy(null, accessAsOwner());

  // route to the relevant graph connection (find or create)
  watch(
    toValueRef(paramsRef),
    async () => {
      // nocheckin
    },
    { immediate: true },
  );

  const roots = graph.getManyRef(computed(() => paramsRef.value.roots));
  return { graph, access, connection: proxyWrap(connection, { name: "connection" }), roots };
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function useSearchNodes<T extends NodeType>(
  params: MaybeRef<SearchConnectionParams<T>>,
): SearchConnectionResult<T> & { connection: GraphConnection<"search", T> } {
  const paramsRef = toRef(params) as Ref<SearchConnectionParams<T>>;
  throw new Error("not yet implemented");
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * TODO :Feature: live aggregation
 */
export function useAggregateNodes(
  params: MaybeRef<AggregateConnectionParams>,
): AggregateConnectionResult & { connection: GraphConnection<"aggregate", NodeType> } {
  const paramsRef = toRef(params) as Ref<AggregateConnectionParams>;
  throw new Error("not yet implemented");
}

/**
 * Gets the current connection for the given scope. Does not acquire any new connections.
 * NOTE: for performance the graph/connection proxies are 'lazy' (batched per tick as regular refs).
 *  That means changing 'node' will change connection/graph only on the next tick.
 */
export function useActiveConnection<T extends NodeType = any>(
  node: MaybeRef<NodeReferenceData | TypedNodeReferenceData<any> | null>,
): {
  graph: ReadNodeGraph;
  connection: GraphConnection<"get", T>;
} {
  const nodeRef = toRef(node) as Ref<NodeReferenceData>;
  const graph = new ProxyNodeGraph(null);
  const connection: ShallowRef<GraphConnection<"get", T> | null> = shallowRef(null);

  // route to the appropriate graph connection
  watch(
    toValueRef(nodeRef),
    () => {
      
      // nocheckin
    },
    { immediate: true },
  );

  return { graph: markRaw(graph), connection: proxyWrap(connection, { name: "connection" }) };
}
