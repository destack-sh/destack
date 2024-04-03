import {
  HUMANIZED_OPERATION_STATUS,
  getGraphClient,
  type GrpcStatusName,
  type OperationMetadata,
} from "@/proto/services";
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
  EditData,
} from "@/proto/wire";
import { describeNode, makeDefaultBenchProto, unwrapSomeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { AccessProxy, accessFromMatrix, accessFull, type AccessArbiter } from "@/system/access";
import { LOCAL_SPACE_PTR, spaceGraphLocal } from "@/system/client";
import { LayerNodeGraph, NodeGraph, ProxyNodeGraph, type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { toaster } from "@/system/toast";
import {
  ImmediateTransactionBuffer,
  canonicalizeEdits,
  editGraph,
  getTransactionBuffer,
  type Transaction,
  type TransactionBuffer,
  RemoteTransactionBuffer,
  newBufferId,
} from "@/system/transaction";
import { AsyncEvent } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
import { log } from "@/utils/log";
import { deepValueEquals, immediateStopWatch, pretendReadonly, toValueRef } from "@/utils/ref";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { useNetwork, whenever } from "@vueuse/core";
import { computed, isRef, shallowRef, toRef, watch, type MaybeRef, type Ref, type ShallowRef } from "vue";

export function makeReadOptions(options: Partial<ReadOptionsData>): ReadOptionsData {
  return {
    ...makeDefaultBenchProto(BenchType.READ_OPTIONS),
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

export type PageInfo = { cursors: string[]; startCursor?: string; size: number; total?: number };

/** The options to the connection supervisor. */
type ConnectionOptions = {
  retryOn?: GrpcStatusName[];
};

const DEFAULT_CONNECTION_OPTIONS: ConnectionOptions = {
  retryOn: ["DEADLINE_EXCEEDED", "UNAVAILABLE", "INTERNAL", "UNKNOWN"],
};

/** Meta-info about the connection. */
type ConnectionMetadata = {
  /** Identifies the request/connection for logging. Client side only. */
  id: number;
  /** Unique name of the connection. */
  name: string;
  /** Whether to stream in live results/edits. */
  live: boolean;
  /** Further options for the underlying connection. */
  options: ConnectionOptions;
  /** Condensed printable current params for debugging. */
  paramsPretty?: Ref<Record<string, any>>;
};

type GetConnectionParams<T extends NodeType> = {
  enabled?: boolean;
  roots: (Omit<NodeReferenceData, "type"> & { type: T })[];
  scope?: Partial<GraphScope>;
  options?: Partial<ReadOptionsData>;
};
type GetConnectionResult<T extends NodeType> = {
  access: AccessArbiter;
  graph: ReadNodeGraph;
  roots: Ref<NodeTypeMapping[T][]>;
};

type SearchConnectionParams<T extends NodeType> = {
  enabled?: boolean;
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
  roots: Ref<TypedNodeReferenceData<T>[]>;
  page: Ref<PageInfo>;
};

type AggregateConnectionParams = {
  enabled?: boolean;
  nodeType: NodeType;
  scope?: Partial<GraphScope>;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  aggregation: ExpressionData;
};
type AggregateConnectionResult = {
  aggregation: Ref<AggregationData>;
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
  if ("roots" in params && params.roots.length > 0) return { benchId: params.roots[0].benchId };
  if ("bases" in params && (params.bases?.length ?? 0) > 0) return { benchId: params.bases![0].benchId };

  throw new Error(`cannot determine scope from params: ${JSON.stringify(params)}`);
}

/** Filter, canonicalize and apply the given edits */
function applyRemoteEdits<T extends NodeType>(
  params: GetConnectionParams<T> | SearchConnectionParams<T>,
  edits: EditData[],
  graph: ReadNodeGraph & WriteNodeGraph,
): void {
  // TODO :Broken: connection 'overlap' detection is broken :ConnectionOverlapFilter
  // node types included?
  const includedNodeTypes = [...(params.options?.ancestorTypes ?? []), ...(params.options?.descendantTypes ?? [])];
  if ("roots" in params) includedNodeTypes.push(...params.roots.map((r) => r.type));
  if ("nodeType" in params) includedNodeTypes.push(params.nodeType);

  const filteredEdits = edits.filter((e) => includedNodeTypes.includes(e.nodeType));
  canonicalizeEdits(Timestamp.now(), filteredEdits);
  editGraph(graph, filteredEdits);
}

//
// Connections
//

export const network = useNetwork();

/** A connection to a subgraph. */
export type GraphConnection<K extends GraphConnectionKind, T extends NodeType> = {
  /** Immutable-after-construction metadata about this connection. */
  readonly meta: ConnectionMetadata;
  /** Immutable-after-construction parameters to this connection. */
  readonly params: ConnectionParamsMapping<T>[K];
  /** The reactive result of this connection. */
  readonly result: ShallowRef<ConnectionResultMapping<T>[K] | null>;
  /** The transaction buffer for editing this subgraph. */
  readonly txBuffer: TransactionBuffer;
  /** An active Transaction for editing this subgraph. */
  readonly tx: Transaction;

  /** Connected to the underlying graph as specified. */
  readonly isConnected: Readonly<Ref<boolean>>;
  /** Currently fetching (or re-fetching) from the underlying graph.  */
  readonly isFetching: Readonly<Ref<boolean>>;
  /** Temporarily paused from re-connecting or receiving live updates (for debugging). */
  readonly isPaused: Readonly<Ref<boolean>>;
  /** Closed and will not re-connect again. */
  readonly isClosed: Readonly<Ref<boolean>>;

  /** Toggle isPaused for debugging. */
  togglePaused(): void;
  /** Waits for a result matching the predicate */
  waitForResult(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void>;
};

type ConnectionInternalResult = { subs?: (() => void)[] };

export abstract class GraphConnectionBase<K extends GraphConnectionKind, T extends NodeType> {
  abstract readonly kind: K;

  readonly meta: ConnectionMetadata;
  readonly params: ConnectionParamsMapping<T>[K];
  readonly result: ShallowRef<(ConnectionResultMapping<T>[K] & ConnectionInternalResult) | null> = shallowRef(null);
  readonly txBuffer: TransactionBuffer;

  readonly isConnected: Ref<boolean> = shallowRef(false);
  readonly isFetching: Ref<boolean> = shallowRef(false);
  readonly isPaused: Ref<boolean> = shallowRef(false);
  readonly isClosed: Ref<boolean> = shallowRef(false);

  private abortController: AbortController | null = null; // for active fetch
  referenceCount: number = 0;

  constructor(meta: ConnectionMetadata, params: ConnectionParamsMapping<T>[K], txBuffer: TransactionBuffer) {
    this.meta = meta;
    this.params = params;
    this.txBuffer = txBuffer;
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

  get tx(): Transaction {
    return this.txBuffer.tx;
  }

  togglePaused(): void {
    this.isPaused.value = !this.isPaused.value;
    log.debug(`graph.${this.kind}.togglePaused`, { name: this.meta.name, id: this.id, paused: this.isPaused.value });
    toaster.debug({
      title: this.isPaused.value ? "Connection paused" : "Connection resumed",
      text: `'${this.kind}:${this.meta.name}' is ${this.isPaused.value ? "not receiving anything" : "receiving data again"}.`,
    });
  }

  get operationMeta(): OperationMetadata {
    return {
      connectionId: this.meta.id,
      operationName: `${this.kind}:${this.meta.name}`,
      suppressErrors: true, // handled by connection
    };
  }

  async waitForResult(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void> {
    return new Promise<void>((resolve) => {
      const stop = immediateStopWatch(this.result, () => {
        if (predicate(this.result.value)) {
          stop();
          resolve();
        }
      });
    });
  }

  /**
   * Maintain this connection until the end of time (or until closed).
   * Immediately tries to fetch. If we fail:
   *   1. If online, try again after an exponential backoff.
   *   2. If offline, try again when we're online.
   * */
  async connect(options?: Partial<ConnectionOptions>): Promise<void> {
    if (this.isConnected.value) throw new Error("already connected");
    options = { ...DEFAULT_CONNECTION_OPTIONS, ...this.meta.options, ...options };

    log.trace(`graph.${this.kind}.connect`, { name: this.meta.name, options });

    const retrySignal = new AsyncEvent();
    let waitingForOnline = false;
    let retryCount = 0;
    let lastErrorCode: string | null = null;

    const shouldRetry = (error: Error) => {
      if (this.isClosed.value) return false;
      if (!(error as RpcError).code) return true; // unknown error
      const rpcError = error as RpcError;
      return options!.retryOn!.includes(rpcError.code as GrpcStatusName);
    };

    const onError = (error: Error) => {
      // notify
      log.error(`graph.${this.kind}.error`, { name: this.meta.name, error });
      const errorCode = (error as RpcError).code ?? "UNKNOWN";
      const retry = shouldRetry(error);
      if (errorCode != lastErrorCode) {
        toaster.error({
          key: `connection:${this.meta.id}`,
          title: HUMANIZED_OPERATION_STATUS[(error as RpcError).code] ?? "Server error",
          text: `'${this.kind}:${this.meta.name}' failed: ${IS_DEBUG ? error.message : (error as RpcError).code}`,
          override: true,
        });
        lastErrorCode = errorCode;
      }

      // (schedule) retry
      if (!retry) {
        log.trace(`graph.${this.kind}.error.unrecoverable`, this.meta.name, error);
        this.isClosed.value = true;
      } else if (network.isOnline.value) {
        retryCount++;
        const delay = Math.min(2 ** (retryCount + 1) * 1000, 60 * 1000);
        setTimeout(() => {
          log.trace(`graph.${this.kind}.retry.backoff`, this.meta.name, { retryCount, delay });
          retrySignal.set();
        }, delay);
      } else {
        waitingForOnline = true;
      }
    };

    whenever(network.isOnline, () => {
      if (network.isOnline.value && waitingForOnline) {
        waitingForOnline = false;
        log.trace(`graph.${this.kind}.retry.online`, this.meta.name);
        retrySignal.set();
      }
    });

    const establishAndMaintainConnection = async () => {
      while (!this.isClosed.value) {
        retrySignal.reset();
        try {
          const newResult = await this.fetch({ ...this.params, onError });
          if (this.result.value != null) {
            this.result.value.subs?.forEach((sub) => sub());
          }
          this.result.value = newResult;

          if (retryCount > 0) {
            toaster.info({
              key: `connection:${this.meta.id}`,
              title: "Reconnected",
              text: `'${this.kind}:${this.meta.name}' reconnected.`,
              override: true,
            });
            retryCount = 0;
            lastErrorCode = null;
          }
          this.isConnected.value = true;
        } catch (error) {
          onError(error as Error);
        }
        await retrySignal.wait();
      }
    };

    establishAndMaintainConnection(); // run async
  }

  /**
   * Fetch the results of this connection for the given params once.
   * If live, also updates the results from the source (until aborted).
   * If there is an error, the fetch is aborted and the onError handler is called.
   * */
  async fetch(
    params: ConnectionParamsMapping<T>[K] & { scope?: GraphScope; onError?: (error: Error) => void },
  ): Promise<ConnectionResultMapping<T>[K] & ConnectionInternalResult> {
    if (this.isFetching.value) this.abortController?.abort();

    log.debug(`graph.${this.kind}`, this.meta.name, params);
    this.isFetching.value = true;
    this.abortController = new AbortController();
    const onError = (error: Error) => {
      this.abortController?.abort();
      params.onError?.(error);
    };

    try {
      const scope = params.scope ?? getScopeFromParams(params);
      const result = await this.doFetch(scope, params, this.abortController.signal, onError);
      this.abortController = null;
      log.debug(`graph.${this.kind}.completed`, this.meta.name, params, result);
      return result;
    } finally {
      if (this.abortController) {
        // cleanup
        this.abortController.abort();
        this.abortController = null;
      }
      this.isFetching.value = false;
    }
  }

  /** Actually fetch in the relevant connection type. */
  protected abstract doFetch(
    scope: GraphScope,
    params: ConnectionParamsMapping<T>[K],
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<ConnectionResultMapping<T>[K] & ConnectionInternalResult>;

  /** Whether this connection is a superset of the given connection */
  supports(params: ConnectionParamsMapping<T>[K]): boolean {
    if (this.kind == "get") {
      // TODO :Broken: connection 'overlap' detection is broken :ConnectionOverlapFilter
      const thisGet = this.params as GetConnectionParams<T>;
      const otherGet = params as GetConnectionParams<T>;
      // scope included?
      if (getScopeFromParams(otherGet).benchId != getScopeFromParams(thisGet).benchId) return false;
      // node types included?
      const thisNodeTypes = [
        ...thisGet.roots.map((r) => r.type),
        ...(thisGet.options?.ancestorTypes ?? []),
        ...(thisGet.options?.descendantTypes ?? []),
      ];
      const otherNodeTypes = [
        ...otherGet.roots.map((r) => r.type),
        ...(otherGet.options?.ancestorTypes ?? []),
        ...(otherGet.options?.descendantTypes ?? []),
      ];
      if (!otherNodeTypes.every((t) => thisNodeTypes.includes(t))) return false;
      return true;
    } else if (this.kind == "search") {
      return deepValueEquals(this.params, params);
    } else if (this.kind == "aggregate") {
      return deepValueEquals(this.params, params);
    } else {
      throw new Error(`unsupported connection kind: ${this.kind}`);
    }
  }
}

export class RemoteGetConnection<T extends NodeType> extends GraphConnectionBase<"get", T> {
  readonly kind = "get";

  protected async doFetch(
    scope: GraphScope,
    params: GetConnectionParams<T>,
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<GetConnectionResult<T> & ConnectionInternalResult> {
    const client = await getGraphClient(scope);
    const graph = new NodeGraph({ scope });
    const options = makeReadOptions(params.options ?? {});
    const subs: (() => void)[] = [];

    // fetch nodes
    const {
      response: { epoch, access: accessMatrix, nodes },
    } = await client.getNodes({ scope: graph.scope, roots: params.roots, options }, { abort, ...this.operationMeta });
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
        { abort, ...this.operationMeta },
      );
      editStream.responses.onNext((rep) => rep != null && applyRemoteEdits(params, rep.edits, graph));
      editStream.responses.onError(onError);
    } else {
      // otherwise directly apply confirmed edits
      subs.push(this.txBuffer.subscribe((edits) => applyRemoteEdits(params, edits, graph)));
    }

    return { graph, access, roots: graph.getManyRef(params.roots), subs };
  }
}

export class RemoteSearchConnection<T extends NodeType> extends GraphConnectionBase<"search", T> {
  readonly kind = "search";

  protected async doFetch(
    scope: GraphScope,
    params: SearchConnectionParams<T>,
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<SearchConnectionResult<T> & ConnectionInternalResult> {
    const client = await getGraphClient(scope);
    const graph = new NodeGraph({ scope });
    const options = makeReadOptions(params.options ?? {});
    const subs: (() => void)[] = [];

    // fetch nodes
    const {
      response: {
        epoch,
        access: accessMatrix,
        nodes,
        roots: rootsInitial,
        cursors: cursorsInitial,
        startCursor: startCursorInitial,
        total: totalInitial,
      },
    } = await client.searchNodes(
      {
        ...params,
        nodeType: params.nodeType,
        bases: params.bases ?? [],
        scope: graph.scope,
        sort: params.sort ?? [],
        options,
      },
      { abort, ...this.operationMeta },
    );
    graph.extend(...nodes.map(unwrapSomeNode));

    // TODO :Incomplete? :Feature: watch search, not just edits to initial results
    const roots = shallowRef(rootsInitial as TypedNodeReferenceData<T>[]);
    const access = accessFromMatrix(accessMatrix!);
    const page = shallowRef({
      cursors: cursorsInitial,
      startCursor: startCursorInitial,
      size: rootsInitial.length,
      total: totalInitial,
    });

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
        { abort, ...this.operationMeta },
      );
      editStream.responses.onNext((rep) => rep != null && applyRemoteEdits(params, rep.edits, graph));
      editStream.responses.onError(onError);
    } else {
      // otherwise directly apply confirmed edits
      subs.push(this.txBuffer.subscribe((edits) => applyRemoteEdits(params, edits, graph)));
    }

    return { graph, access, roots, page, subs };
  }
}

export class LocalGetConnection<T extends NodeType> extends GraphConnectionBase<"get", T> {
  readonly kind = "get";

  readonly graph: ReadNodeGraph;

  constructor(meta: ConnectionMetadata, params: GetConnectionParams<T>, graph: WriteNodeGraph & ReadNodeGraph) {
    super(meta, params, new ImmediateTransactionBuffer(newBufferId(), graph.scope, graph));
    this.graph = graph;

    // 'fuse' the connection
    this.isConnected.value = true;
    this.result.value = { graph: this.graph, access: accessFull(), roots: this.graph.getManyRef(params.roots) };
  }

  connect(options?: Partial<ConnectionOptions> | undefined): Promise<void> {
    // no-op, already fused
    return Promise.resolve();
  }

  protected async doFetch(scope: GraphScope, params: GetConnectionParams<T>): Promise<GetConnectionResult<T>> {
    return { graph: this.graph, access: accessFull(), roots: this.graph.getManyRef(params.roots) };
  }
}

/** Shallow reactive proxy for a deferred connection. */
export class ProxyConnection<K extends GraphConnectionKind, T extends NodeType> implements GraphConnection<K, T> {
  connection: ShallowRef<GraphConnectionBase<K, T> | null>;
  readonly isConnected: Ref<boolean>;
  readonly isFetching: Ref<boolean>;
  readonly isPaused: Ref<boolean>;
  readonly isClosed: Ref<boolean>;

  constructor(connection: MaybeRef<GraphConnectionBase<K, T> | null>) {
    this.connection = isRef(connection) ? connection : shallowRef(connection);
    this.isConnected = computed(() => this.connection.value?.isConnected.value ?? false);
    this.isFetching = computed(() => this.connection.value?.isFetching.value ?? false);
    this.isPaused = computed(() => this.connection.value?.isPaused.value ?? false);
    this.isClosed = computed(() => this.connection.value?.isClosed.value ?? false);
  }

  togglePaused(): void {
    this.activeConnection.togglePaused();
  }

  async waitForResult(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void> {
    return new Promise((resolve) => {
      immediateStopWatch(
        () => this.connection.value?.result.value,
        (stop) => {
          if (predicate(this.connection.value?.result.value ?? null)) {
            stop();
            resolve();
          }
        },
      );
    });
  }

  get activeConnection(): GraphConnectionBase<K, T> {
    if (this.connection.value == null) throw new Error("no active connection");
    return this.connection.value;
  }

  get meta(): ConnectionMetadata {
    return this.activeConnection.meta;
  }

  get params(): ConnectionParamsMapping<T>[K] {
    return this.activeConnection.params;
  }

  get result(): ShallowRef<ConnectionResultMapping<T>[K] | null> {
    return this.activeConnection.result;
  }

  get txBuffer(): TransactionBuffer {
    return this.activeConnection.txBuffer;
  }

  get tx(): Transaction {
    return this.activeConnection.tx;
  }
}

//
// Maintaining and routing connections
//

let connectionId = 0;
function newConnectionId(): number {
  return connectionId++;
}
const _graphConnections: Ref<GraphConnectionBase<any, any>[]> = shallowRef([
  // add local graph
  new LocalGetConnection(
    { id: newConnectionId(), name: "local.space", live: true, options: {} },
    { roots: [LOCAL_SPACE_PTR], options: makeReadOptions({ descendantTypes: [NodeType.VIEW] }) },
    spaceGraphLocal as WriteNodeGraph & ReadNodeGraph, // we export it as read-only but it's actually writable
  ),
]);
export const graphConnections = pretendReadonly(_graphConnections);

export function addGraphConnection(connection: GraphConnectionBase<any, any>): void {
  _graphConnections.value = [..._graphConnections.value, connection];
}

/** RC-=1. Connections without references are GCed after some time. */
function releaseConnection(connection: GraphConnectionBase<any, any>): void {
  connection.referenceCount--;
  // TODO :Broken: GC connections without references after some time
  // TODO :Performance: cache/store connections (results) locally for initial hydration
}

/** Finds an existing connection and acquires it (RC+=1) */
function acquireExistingConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  params: ConnectionParamsMapping<T>[K],
): GraphConnectionBase<K, T> | null {
  const connection = _graphConnections.value.find((c) => c.kind == kind && c.supports(params)) ?? null;
  if (connection) connection.referenceCount++;
  return connection;
}

type ConnectionMetadataIn = Pick<ConnectionMetadata, "name"> & Partial<ConnectionMetadata>;

/** Creates a new (remote) connection and immediately acquires it. */
async function acquireNewConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  metaIn: Pick<ConnectionMetadata, "name"> & Partial<ConnectionMetadata>,
  params: ConnectionParamsMapping<T>[K],
): Promise<GraphConnectionBase<K, T>> {
  const meta: ConnectionMetadata = { live: false, id: newConnectionId(), options: {}, ...metaIn };

  // create
  const scope = getScopeFromParams(params);
  const txBuffer = await getTransactionBuffer(scope);
  let connection: GraphConnectionBase<K, T>;
  if (kind == "get") {
    const getParams = params as GetConnectionParams<T>;
    connection = new RemoteGetConnection<T>(meta, getParams, txBuffer) as any as GraphConnectionBase<K, T>;
  } else if (kind == "search") {
    const searchParams = params as SearchConnectionParams<T>;
    connection = new RemoteSearchConnection<T>(meta, searchParams, txBuffer) as any as GraphConnectionBase<K, T>;
  } else {
    throw new Error(`unsupported connection kind: ${kind}`);
  }
  addGraphConnection(connection);
  connection.referenceCount++;

  // connect
  await connection.connect(meta.options);

  return connection;
}

/** Gets or acquires a connection given the params, maintaining reference counts and such. */
export function useConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<ConnectionParamsMapping<T>[K]>,
): Ref<GraphConnectionBase<K, T> | null> {
  const connection: ShallowRef<GraphConnectionBase<K, T> | null> = shallowRef(null);
  const paramsRef = toRef(params) as Ref<ConnectionParamsMapping<T>[K]>;

  // acquire existing or create new connection
  watch(
    toValueRef(paramsRef),
    async () => {
      const old = connection.value;
      if (old) {
        releaseConnection(old);
        connection.value = null;
      }
      if (paramsRef.value.enabled === false) return; // disabled

      // if the existing connection can support the new query, we'll just acquire it again
      const existing = acquireExistingConnection(kind, paramsRef.value);
      if (existing) connection.value = existing;
      else connection.value = await acquireNewConnection(kind, metaIn, paramsRef.value);
    },
    { immediate: true },
  );

  return connection;
}

/**
 * Gets the current connection for the given scope. Does not acquire any new connections.
 * NOTE: for performance the graph/connection proxies are 'lazy' (batched per tick as regular refs).
 *  That means changing 'node' will change connection/graph only on the next tick.
 */
export function useExistingConnection<T extends NodeType = any>(
  node: MaybeRef<NodeReferenceData | TypedNodeReferenceData<any> | null>,
  options?: {
    isGlobal?: boolean;
  },
): {
  graph: ReadNodeGraph;
  connection: GraphConnection<"get", T>;
} {
  const nodeRef = toRef(node) as Ref<NodeReferenceData>;
  const graph = new ProxyNodeGraph(null);
  const connection: ShallowRef<GraphConnectionBase<"get", T> | null> = shallowRef(null);

  // route to the appropriate connection
  const refreshConnection = () => {
    const oldConnection = connection.value;
    let newConnection = null;
    if (connection.value) {
      releaseConnection(connection.value);
    }
    if (nodeRef.value != null) {
      newConnection = acquireExistingConnection("get", { roots: [nodeRef.value as TypedNodeReferenceData<T>] });
      if (newConnection == null && !options?.isGlobal)
        throw new Error(
          `missing connection for ${describeNode(nodeRef.value)} (available: ${_graphConnections.value.map((c) => c.name)})`,
        );
    }
    if (newConnection !== oldConnection) {
      connection.value = newConnection as GraphConnectionBase<"get", T> | null;
    }
    graph._graph.value = connection.value?.result.value?.graph ?? null;
  };
  watch(toValueRef(nodeRef), refreshConnection, { immediate: true });

  // NOTE: useExistingConnection is mostly used where a connection must exist (inside View components).
  //  Otherwise if we don't have a connection we need to check *every* new connection if it's a match (until we have one).
  if (options?.isGlobal) {
    let stopGlobalWatch = null as (() => void) | null;
    watch(
      connection,
      () => {
        stopGlobalWatch?.();
        if (!connection.value) {
          stopGlobalWatch = watch(_graphConnections, refreshConnection);
        }
      },
      { immediate: true },
    );
  }

  // sync result
  watch(
    () => connection.value?.result.value,
    () => (graph.graph = connection.value?.result.value?.graph ?? null),
  );

  return { graph, connection: new ProxyConnection(connection) };
}

// nocheckin: use tx buffer overlay (and filter) in connections
/** The graph of a node connection overlaid with its local buffer */
function connectionOverlayGraph<T extends NodeType>(
  connection: Ref<GraphConnectionBase<"get" | "search", T> | null>,
): ReadNodeGraph {
  const graph = new LayerNodeGraph([]);
  watch(
    () => connection.value?.result.value,
    () => {
      if (connection.value?.result.value == null) {
        graph.layers.value = [];
      } else {
        graph.layers.value = [connection.value!.result.value!.graph, connection.value!.txBuffer.overlay];
      }
    },
    { immediate: true },
  );
  return graph;
}

/**
 * Gets the given nodes from the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that edits for the given nodes are watched.
 */
export function useGetNodes<T extends NodeType>(
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<GetConnectionParams<T>>,
): GetConnectionResult<T> & { connection: GraphConnection<"get", T> } {
  const paramsRef = toRef(params) as Ref<GetConnectionParams<T>>;
  const connection = useConnection<"get", T>("get", metaIn, paramsRef);

  // map results
  // TODO :Cleanup: mapping connection results is a deep ref chain?
  const graph = connectionOverlayGraph(connection);
  const access = new AccessProxy(
    computed(() => connection.value?.result?.value?.access ?? null),
    { default: accessFull() },
  );
  const roots: Ref<NodeTypeMapping[T][]> = computed(() => connection.value?.result.value?.roots?.value ?? []);

  return {
    graph,
    access,
    connection: new ProxyConnection(connection),
    roots,
  };
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 */
export function useSearchNodes<T extends NodeType>(
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<SearchConnectionParams<T>>,
): SearchConnectionResult<T> & { connection: GraphConnection<"search", T> } {
  const paramsRef = toRef(params) as Ref<SearchConnectionParams<T>>;
  const connection = useConnection<"search", T>("search", metaIn, paramsRef);

  // map results
  const graph = connectionOverlayGraph(connection);
  const access = new AccessProxy(
    computed(() => connection.value?.result?.value?.access ?? null),
    { default: accessFull() },
  );
  const roots: Ref<TypedNodeReferenceData<T>[]> = computed(() => connection.value?.result?.value?.roots?.value ?? []);
  const page: Ref<PageInfo> = computed(
    () => connection.value?.result?.value?.page?.value ?? ({ cursors: [], size: 0 } as PageInfo),
  );

  return {
    graph,
    access,
    connection: new ProxyConnection(connection),
    roots,
    page,
  };
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * TODO :Feature: live aggregation
 */
export function useAggregateNodes(
  params: MaybeRef<AggregateConnectionParams>,
): AggregateConnectionResult & { connection: GraphConnectionBase<"aggregate", NodeType> } {
  throw new Error("aggregate not yet implemented");
}
