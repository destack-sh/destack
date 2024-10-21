import { SOURCE_NODE_TYPES } from "@/language/const";
import {
  DEFAULT_NODE_FILTER,
  LayerNodeGraph,
  NodeGraph,
  NodeSuperGraph,
  ProxyNodeGraph,
  type ReadNodeGraph,
  type WriteNodeGraph,
} from "@/language/graph";
import {
  ImmediateTransactionBuffer,
  editGraph,
  getTransactionBuffer,
  newBufferId,
  type Transaction,
  type TransactionBuffer,
} from "@/language/transaction";
import {
  HUMANIZED_OPERATION_STATUS,
  getGraphClient,
  getGraphTransport,
  type GrpcStatusName,
  type OperationMetadata,
} from "@/proto/services";
import {
  AggregationResultData,
  ExpressionData,
  NodeType,
  ObjectType,
  SelectOptionsData,
  type GraphScopeData,
  type NodeReferenceData,
  type NodeTypeMapping,
} from "@/proto/wire";
import { HealthClient } from "@/proto/wire/proto/health.client";
import {
  EMPTY_SCOPE,
  SomeNodeReferenceData,
  deepContentEquals,
  describeNode,
  makeDefaultObject,
  makeScope,
  unwrapSomeNode,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { LOCAL_SPACE_PTR, PACKAGE_SCOPE, packagePtr, spaceGraphLocal } from "@/system/client";
import { toaster } from "@/ui/toast";
import { AsyncEvent } from "@/utils/functools";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { immediateStopWatch, pretendReadonly, toValueRef } from "@/utils/ref";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { tryOnBeforeUnmount, useInterval, useNetwork, whenever } from "@vueuse/core";
import { DateTime } from "luxon";
import {
  computed,
  isRef,
  markRaw,
  ref,
  shallowRef,
  toRef,
  toValue,
  triggerRef,
  watch,
  type MaybeRef,
  type Ref,
  type ShallowRef,
} from "vue";

export function getScopeKey(scope: GraphScopeData): string {
  return JSON.stringify(scope);
}

//
// Connection typing.
// NOTE: we re-type the graph connection params here to relax some constraints for convenience
//

export type PageInfo = { size: number; total?: number };

/** The options to the connection supervisor. */
type ConnectionOptions = {
  shouldRetry: (error: RpcError) => boolean;
};

const DEFAULT_CONNECTION_OPTIONS: ConnectionOptions = {
  shouldRetry: (error) => {
    return (
      ["DEADLINE_EXCEEDED", "UNAVAILABLE", "INTERNAL", "UNKNOWN"].includes(error.code) ||
      error.message?.includes("missing trailer") ||
      error.message?.includes("missing header")
    );
  },
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
  options: Partial<ConnectionOptions>;
  /** Condensed printable current params for debugging. */
  paramsPretty?: Ref<Record<string, any>>;
};

// get connection
export type GetConnectionParams<T extends NodeType> = {
  isEnabled?: boolean;
  scope: GraphScopeData;
  roots: (Omit<NodeReferenceData, "type"> & { nodeType: T })[];
  blockPtr?: NodeReferenceData;
  isOptional?: boolean;
  ancestorTypes?: NodeType[];
  descendantTypes?: NodeType[];
  select?: Partial<SelectOptionsData>;
  includeDeleted?: boolean;
};
export type GetConnectionResult<T extends NodeType> = {
  graph: ReadNodeGraph;
  overlay: ReadNodeGraph | null;
  roots: Ref<NodeTypeMapping[T][]>;
  epoch: Ref<bigint>;
};

// search connection
export type SearchConnectionParams<T extends NodeType> = {
  isEnabled?: boolean;
  scope: GraphScopeData;
  nodeType: T;
  blockPtr?: NodeReferenceData;
  filter?: ExpressionData;
  sort?: ExpressionData[];
  ancestorTypes?: NodeType[];
  descendantTypes?: NodeType[];
  first?: number;
  skip?: number;
  count?: boolean;
  select?: Partial<SelectOptionsData>;
};
export type SearchConnectionResult<T extends NodeType> = {
  graph: ReadNodeGraph;
  overlay: ReadNodeGraph | null;
  rootsPtr: Ref<TypedNodeReferenceData<T>[]>;
  page: Ref<PageInfo>;
  epoch: Ref<bigint>;
};

// aggregate connection
export type AggregateConnectionParams = {
  isEnabled?: boolean;
  scope: GraphScopeData;
  nodeType: NodeType;
  bases?: NodeReferenceData[];
  filter?: ExpressionData;
  sort?: ExpressionData[];
  aggregation: ExpressionData;
};
export type AggregateConnectionResult = {
  aggregation: Ref<AggregationResultData>;
  epoch: Ref<bigint>;
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

function getScopeFromParams<T extends NodeType>(
  params: ConnectionParamsMapping<T>[GraphConnectionKind],
): GraphScopeData {
  if (params.scope != null) return params.scope;
  if ("roots" in params && params.roots.length > 0) return makeScope({ benchId: params.roots[0].benchId });
  if ("bases" in params && (params.bases?.length ?? 0) > 0) return makeScope({ benchId: params.bases![0].benchId });
  // NOTE: scope defaults to current package (not sure if this is right.. probably want to make it explicit)
  return makeScope({ benchId: packagePtr.value?.benchId });
}

function getNodeTypesFromParams<T extends NodeType>(
  params: ConnectionParamsMapping<T>[GraphConnectionKind],
): NodeType[] {
  const nodeTypes: NodeType[] = [];
  if ("roots" in params) nodeTypes.push(...params.roots.map((r) => r.nodeType));
  if ("nodeType" in params) nodeTypes.push(params.nodeType);
  if ("ancestorTypes" in params && params.ancestorTypes) {
    nodeTypes.push(...params.ancestorTypes);
  }
  if ("descendantTypes" in params && params.descendantTypes) {
    nodeTypes.push(...params.descendantTypes);
  }
  return nodeTypes;
}

/**
 * Derives the overlay graph for a specific connection
 *
 * NOTE :UX :Architecture: instead of ignoring transactions from other connections outright we could optimistically
 *  apply edits to the same node identities to other connections as well. Ultimately, for better responsiveness,
 *  we probably want to emulate even more of the backend Connection behavior (e.g., optimistic search results).
 * (the reason for having the connection filter below is that while the backend properly filters edits per connection,
 *  here we distribute optimistic edits across the per-bench tx buffer, so an edit might not be relevant or even valid)
 */
function makeConnectionOverlayGraph(
  base: NodeGraph,
  connection: Connection<any, any>,
): { graph: NodeGraph; sub: () => void } {
  const overlay = new NodeGraph({ scope: base.scope, nodeTypes: base.nodeTypes, isOverlayOf: base });
  // add current buffered edits
  {
    const currentBuffered = connection.txBuffer.getBuffered();
    const currentEdits = [...(currentBuffered[-1] ?? []), ...(currentBuffered[connection.meta.id] ?? [])];
    editGraph(overlay, currentEdits, { base: base });
  }
  // susbcribe to buffer changes
  const sub = connection.txBuffer.subscribeBuffered((event) => {
    if (event.meta.connectionId == null || event.meta.connectionId == connection.meta.id) {
      if (event.type == "reset") {
        overlay.clear();
      }
      if (event.newEdits.length > 0) {
        editGraph(overlay, event.newEdits, { base: base });
      }
    }
  });
  return { graph: overlay, sub };
}

//
// Connections
//

export const network = useNetwork();

/** A connection to a subgraph. */
export type Connection<K extends GraphConnectionKind, T extends NodeType> = {
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
  /** When this connection was created */
  readonly createdAt: DateTime;
  /** When this connection was last used (had any active active referents) */
  readonly lastReferencedAt: DateTime | null;

  /** Connected to the underlying graph as specified. */
  readonly isConnected: Readonly<Ref<boolean>>;
  /** Currently fetching (or re-fetching) from the underlying graph.  */
  readonly isConnecting: Readonly<Ref<boolean>>;
  /** Temporarily paused from re-connecting and receiving live updates (for debugging). */
  readonly isPaused: Readonly<Ref<boolean>>;
  /** Closed and will not re-connect again. */
  readonly isClosed: Readonly<Ref<boolean>>;
  /** The last error that caused this connection to be in an error state. */
  readonly lastError: Readonly<Ref<RpcError | null>>;

  /** Increment reference count */
  incRefCount: () => void;
  /** Decrement reference count */
  decRefCount: () => void;
  /** Closes this connection forever. */
  close(): Promise<void>;
  /** Toggle isPaused for debugging. */
  togglePaused(): void;
  /** Get notified on errors */
  onError: (handler: (status: GrpcStatusName) => void) => void;
  /** Waits for a result matching the predicate */
  waitForResult(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void>;
};

type ConnectionInternalResult = { subs?: (() => void)[] };

export abstract class ConnectionBase<K extends GraphConnectionKind, T extends NodeType> {
  abstract readonly kind: K;

  readonly meta: ConnectionMetadata;
  readonly params: ConnectionParamsMapping<T>[K];
  readonly result: ShallowRef<(ConnectionResultMapping<T>[K] & ConnectionInternalResult) | null> = shallowRef(null);
  readonly txBuffer: TransactionBuffer;
  readonly createdAt: DateTime = DateTime.now();
  lastReferencedAt: DateTime | null = null;

  readonly isConnected: Ref<boolean> = ref(false);
  readonly isConnecting: Ref<boolean> = ref(false);
  readonly isPaused: Ref<boolean> = ref(false);
  readonly isClosed: Ref<boolean> = ref(false);
  readonly isError: Ref<boolean> = ref(false);
  readonly lastError: Ref<RpcError | null> = ref(null);

  private onErrorSubs: ((status: GrpcStatusName) => void)[] = [];
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
    return this.txBuffer.tx.with({ connectionId: this.meta.id });
  }

  get epoch(): bigint | null {
    return this.result.value?.epoch.value ?? null;
  }

  incRefCount(): void {
    this.referenceCount++;
    this.lastReferencedAt = DateTime.now();
  }

  decRefCount(): void {
    this.referenceCount--;
    this.lastReferencedAt = DateTime.now();
  }

  async close(): Promise<void> {
    this.isClosed.value = true;
    log.debug(`graph.${this.kind}.close`, { name: this.meta.name, id: this.id });
    this.abortController?.abort();
  }

  togglePaused(): void {
    this.isPaused.value = !this.isPaused.value;
    log.debug(`graph.${this.kind}.togglePaused`, { name: this.meta.name, id: this.id, paused: this.isPaused.value });
    toaster.debug({
      title: this.isPaused.value ? "Connection paused" : "Connection resumed",
      text: `'${this.kind}:${this.meta.name}' is ${this.isPaused.value ? "disconnected" : "reconnected"}.`,
      override: `connection.togglePaused:${this.meta.id}`,
    });
  }

  get operationMeta(): OperationMetadata<any> {
    return {
      connectionId: this.meta.id,
      operationName: `${this.kind}:${this.meta.name}`,
      suppressErrors: true, // handled by connection
    };
  }

  onError(handler: (status: GrpcStatusName) => void): () => void {
    this.onErrorSubs.push(handler);
    return () => {
      const index = this.onErrorSubs.indexOf(handler);
      if (index >= 0) this.onErrorSubs.splice(index, 1);
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
   * Immediately tries to fetch, returns once successfully connected. If we fail:
   *   1. If online, try again after an exponential backoff.
   *   2. If offline, try again when we're online.
   * */
  async connect(options?: Partial<ConnectionOptions>): Promise<void> {
    if (this.isConnected.value) throw new Error("already connected");
    options = { ...DEFAULT_CONNECTION_OPTIONS, ...this.meta.options, ...options };

    log.trace(`graph.${this.kind}.connect`, { name: this.meta.name, options });

    const retrySignal = new AsyncEvent();
    const connectedSignal = new AsyncEvent();
    let waitingForOnline = false;
    let retryCount = 0;
    let lastErrorCode: string | null = null;

    const shouldRetry = (error: Error) => {
      if (this.isClosed.value) return false;
      if (!(error as RpcError).code) return true; // unknown error
      const rpcError = error as RpcError;
      return options?.shouldRetry!(rpcError);
    };

    const onError = (error: Error) => {
      // notify
      this.lastError.value = error as RpcError;
      log.error(`graph.${this.kind}.error`, { name: this.meta.name, error });
      const errorCode = (error as RpcError).code ?? "UNKNOWN";
      if (errorCode) {
        this.onErrorSubs.forEach((sub) => sub(errorCode as GrpcStatusName));
      }
      const retry = shouldRetry(error);
      if (errorCode != lastErrorCode) {
        const op = `${this.kind}:${this.meta.name}`;
        toaster.error({
          title: `'${op}' connection ${retry ? "lost" : "failed"}`,
          text: `'${op}' failed: ${IS_DEV ? error.message : (error as RpcError).code}`,
          override: `connection:${this.meta.id}`,
          summarize: {
            key: "connection.error",
            info: [{ op, error: error as RpcError }],
            title: (infos) => `${infos.length} connections ${retry ? "lost" : "failed"}`,
            text: (infos) => {
              // distinct errors
              const errors = new Set(
                infos
                  .map((info) => HUMANIZED_OPERATION_STATUS[info.error.code])
                  .filter((e) => e != null && e.length > 0),
              );
              const errorStr = [...errors].join(", ") ?? "Unknown error";
              return `${errorStr}: ${infos.map((info) => `'${info.op}'`).join(", ")}`;
            },
          },
        });
        lastErrorCode = errorCode;
      }

      // (schedule) retry
      if (!retry) {
        log.error(`graph.${this.kind}.error.unrecoverable`, this.meta.name, error);
        this.isConnected.value = false;
        this.isClosed.value = true;
      } else if (network.isOnline.value) {
        retryCount++;
        const delay = Math.min(2 ** (retryCount + 1) * 1000, 60 * 1000);
        setTimeout(() => {
          log.trace(`graph.${this.kind}.retry.backoff`, this.meta.name, { retryCount, delay });
          retrySignal.resolve();
        }, delay);
      } else {
        waitingForOnline = true;
      }
    };

    whenever(network.isOnline, () => {
      if (network.isOnline.value && waitingForOnline) {
        waitingForOnline = false;
        log.trace(`graph.${this.kind}.retry.online`, this.meta.name);
        retrySignal.resolve();
      }
    });

    const establishAndMaintainConnection = async () => {
      while (!this.isClosed.value) {
        retrySignal.reset();
        try {
          // (re)connect once
          if (this.isConnecting.value) this.abortController?.abort();
          log.debug(`graph.${this.kind}`, this.meta.name, this.params);
          this.isConnecting.value = true;
          this.abortController = new AbortController();
          let newResult;
          try {
            newResult = await this.doConnect(
              getScopeFromParams(this.params),
              getNodeTypesFromParams(this.params),
              this.params,
              this.abortController.signal,
              (e) => {
                onError(e);
                this.abortController?.abort();
              },
            );
            connectedSignal.resolve();
            this.abortController = null;
            log.debug(`graph.${this.kind}.complete`, this.meta.name, this.params, newResult);
          } finally {
            if (this.abortController) {
              // cleanup
              this.abortController.abort();
              this.abortController = null;
            }
            this.isConnecting.value = false;
          }
          if (this.result.value != null) {
            this.result.value.subs?.forEach((sub) => sub());
          }
          this.result.value = newResult;

          // retry if needed
          if (retryCount > 0) {
            toaster.info({
              title: "Reconnected",
              text: `'${this.kind}:${this.meta.name}' connection restored.`,
              override: `connection:${this.meta.id}`,
              summarize: {
                key: "connection.reconnected",
                info: [{ name: this.meta.name }],
                title: (infos) => `${infos.length} connections restored`,
                text: (infos) => {
                  return infos.map((info) => `'${info.name}'`).join(", ");
                },
              },
            });
            retryCount = 0;
            lastErrorCode = null;
          }
          this.isConnected.value = true;
          this.lastError.value = null;
        } catch (error) {
          onError(error as Error);
          this.isConnected.value = false;
        }
        await retrySignal.wait();
      }
    };

    establishAndMaintainConnection(); // run async
    await connectedSignal.wait(); // wait for first connection
  }

  /** Actually fetch in the relevant connection type. */
  protected abstract doConnect(
    scope: GraphScopeData,
    nodeTypes: NodeType[],
    params: ConnectionParamsMapping<T>[K],
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<ConnectionResultMapping<T>[K] & ConnectionInternalResult>;

  /** Whether this connection is a superset of the given connection */
  includes(params: ConnectionParamsMapping<T>[K]): boolean {
    // NOTE :Broken: connection 'overlap' detection is broken :ConnectionMatching
    //  (but shouldn't be an issue for now as we we fetch the entire package source / other search connections separately)
    if (this.kind == "get") {
      const thisGet = this.params as GetConnectionParams<T>;
      const otherGet = params as GetConnectionParams<T>;
      if (getScopeFromParams(otherGet).benchId != getScopeFromParams(thisGet).benchId) {
        // different bench
        return false;
      }
      const thisNodeTypes = [
        ...thisGet.roots.map((r) => r.nodeType),
        ...(thisGet.ancestorTypes ?? []),
        ...(thisGet.descendantTypes ?? []),
      ];
      if (thisNodeTypes.some((t) => !SOURCE_NODE_TYPES.includes(t))) {
        // we can only assume that node type overlap is enough for source nodes, otherwise check full params
        //  (since they're loaded in a very specific way)
        return deepContentEquals(params, this.params);
      }
      const otherNodeTypes = [
        ...otherGet.roots.map((r) => r.nodeType),
        ...(otherGet.ancestorTypes ?? []),
        ...(otherGet.descendantTypes ?? []),
      ];
      // check whether all the node types overlap
      return otherNodeTypes.every((t) => thisNodeTypes.includes(t));
    } else if (this.kind == "search") {
      return deepContentEquals(params, this.params);
    } else if (this.kind == "aggregate") {
      return deepContentEquals(params, this.params);
    } else {
      throw new Error(`unsupported connection kind: ${this.kind}`);
    }
  }
}

export class RemoteGetConnection<T extends NodeType> extends ConnectionBase<"get", T> {
  readonly kind = "get";

  protected async doConnect(
    scope: GraphScopeData,
    nodeTypes: NodeType[],
    params: GetConnectionParams<T>,
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<GetConnectionResult<T> & ConnectionInternalResult> {
    const client = await getGraphClient(scope);
    const graph = new NodeGraph({ scope, nodeTypes });
    const subs: (() => void)[] = [];
    const select: SelectOptionsData | undefined =
      params.select != null ? makeDefaultObject({ ...params.select, metatype: ObjectType.SELECT_OPTIONS }) : undefined;

    // fetch nodes
    const {
      response: { epoch: initialEpoch, nodes, connectionToken },
    } = await client.getNodes(
      {
        scope: graph.scope,
        roots: params.roots,
        blockPtr: params.blockPtr,
        ancestorTypes: params.ancestorTypes ?? [],
        descendantTypes: params.descendantTypes ?? [],
        select: select,
        isOptional: params.isOptional,
      },
      { abort, ...this.operationMeta },
    );
    graph.extend(...nodes.map(unwrapSomeNode));
    const epoch = ref(initialEpoch);

    // watch edits if live
    if (this.isLive) {
      const editStream = client.watchGet(
        { scope: graph.scope, connectionToken, sinceEpoch: initialEpoch },
        { abort, ...this.operationMeta },
      );
      editStream.responses.onNext((rep) => {
        if (rep == null) return;
        if (rep.epoch < epoch.value) throw new Error(`epoch regression: ${epoch.value} -> ${rep.epoch}`); // sanity check
        epoch.value = rep.epoch;
        editGraph(graph, [...rep.edits, ...rep.cascadedEdits]);
        this.txBuffer.acceptCommitted(rep.edits, rep.cascadedEdits);
      });
      editStream.responses.onError(onError);
      editStream.responses.onComplete(() => onError(new Error("edit stream closed")));
    }

    const { graph: overlay, sub } = makeConnectionOverlayGraph(graph, this);
    subs.push(sub);
    return { graph, overlay, roots: graph.getManyRef(params.roots), epoch, subs };
  }
}

export class RemoteSearchConnection<T extends NodeType> extends ConnectionBase<"search", T> {
  readonly kind = "search";

  protected async doConnect(
    scope: GraphScopeData,
    nodeTypes: NodeType[],
    params: SearchConnectionParams<T>,
    abort: AbortSignal,
    onError: (error: Error) => void,
  ): Promise<SearchConnectionResult<T> & ConnectionInternalResult> {
    const client = await getGraphClient(scope);
    const graph = new NodeGraph({ scope, nodeTypes });
    const select: SelectOptionsData | undefined =
      params.select != null ? makeDefaultObject({ ...params.select, metatype: ObjectType.SELECT_OPTIONS }) : undefined;
    const subs: (() => void)[] = [];

    // fetch nodes
    const {
      response: { epoch: initialEpoch, nodes, rootsPtr: rootsInitial, total: totalInitial, connectionToken },
    } = await client.searchNodes(
      {
        ...params,
        nodeType: params.nodeType,
        blockPtr: params.blockPtr,
        scope: graph.scope,
        sort: params.sort ?? [],
        ancestorTypes: params.ancestorTypes ?? [],
        descendantTypes: params.descendantTypes ?? [],
        select: select,
      },
      { abort, ...this.operationMeta },
    );
    graph.extend(...nodes.map(unwrapSomeNode));
    const epoch = ref(initialEpoch);

    const rootsPtr = shallowRef(rootsInitial as TypedNodeReferenceData<T>[]);
    const page: Ref<PageInfo> = shallowRef({ size: rootsInitial.length, total: totalInitial });

    // watch edits if live
    // NOTE :UX: search should react to current overlay graph (including 'phantom' edits like Logs)
    if (this.isLive) {
      const editStream = client.watchSearch(
        { connectionToken, sinceEpoch: initialEpoch, scope: graph.scope },
        { abort, ...this.operationMeta },
      );
      editStream.responses.onNext((rep) => {
        if (rep == null) return;
        if (rep.epoch < epoch.value) throw new Error(`epoch regression: ${epoch.value} -> ${rep.epoch}`); // sanity check
        epoch.value = rep.epoch;
        // apply other added/removed nodes
        for (const node of rep.addedNodes) {
          graph.add(unwrapSomeNode(node));
        }
        for (const nodePtr of rep.removedNodesPtr) {
          const node = graph.get(nodePtr);
          if (node != null) graph.remove(node);
        }
        // update' roots' list
        rootsPtr.value = rep.rootsPtr as TypedNodeReferenceData<T>[];
        page.value = { size: rep.rootsPtr.length, total: rep.total };
        // apply edits
        editGraph(graph, [...rep.edits, ...rep.cascadedEdits]);
        this.txBuffer.acceptCommitted(rep.edits, rep.cascadedEdits);
      });
      editStream.responses.onError(onError);
      editStream.responses.onComplete(() => onError(new Error("edit stream closed")));
    }

    const { graph: overlay, sub } = makeConnectionOverlayGraph(graph, this);
    subs.push(sub);
    return { graph, overlay, rootsPtr, page, subs, epoch };
  }
}

export class LocalGetConnection<T extends NodeType> extends ConnectionBase<"get", T> {
  readonly kind = "get";

  readonly graph: ReadNodeGraph;

  constructor(
    meta: ConnectionMetadata,
    params: GetConnectionParams<T>,
    readGraph: ReadNodeGraph,
    writeGraph: ReadNodeGraph & WriteNodeGraph,
  ) {
    super(meta, params, new ImmediateTransactionBuffer(newBufferId(), readGraph.scope, writeGraph));
    this.graph = readGraph;

    // 'fuse' the connection
    // (no overlay because the local connection is instant)
    this.isConnected.value = true;
    this.result.value = {
      graph: this.graph,
      overlay: null,
      roots: this.graph.getManyRef(params.roots),
      epoch: ref(-1n),
    };
  }

  connect(options?: Partial<ConnectionOptions> | undefined): Promise<void> {
    // no-op, already fused
    return Promise.resolve();
  }

  protected async doConnect(
    scope: GraphScopeData,
    nodeTypes: NodeType[],
    params: GetConnectionParams<T>,
  ): Promise<GetConnectionResult<T>> {
    return { graph: this.graph, overlay: null, roots: this.graph.getManyRef(params.roots), epoch: ref(-1n) };
  }
}

/** Shallow reactive proxy for a deferred connection. */
export class ProxyConnection<K extends GraphConnectionKind, T extends NodeType> implements Connection<K, T> {
  connection: ShallowRef<ConnectionBase<K, T> | null>;
  readonly createdAt: DateTime = DateTime.now();
  readonly isConnected: Ref<boolean>;
  readonly isConnecting: Ref<boolean>;
  readonly isPaused: Ref<boolean>;
  readonly isClosed: Ref<boolean>;
  readonly lastError: Ref<RpcError | null>;

  constructor(connection: MaybeRef<ConnectionBase<K, T> | null>) {
    this.connection = isRef(connection) ? connection : shallowRef(connection);
    this.isConnected = computed(() => this.connection.value?.isConnected.value ?? false);
    this.isConnecting = computed(() => this.connection.value?.isConnecting.value ?? false);
    this.isPaused = computed(() => this.connection.value?.isPaused.value ?? false);
    this.isClosed = computed(() => this.connection.value?.isClosed.value ?? false);
    this.lastError = computed(() => this.connection.value?.lastError.value ?? null);
  }

  get lastReferencedAt(): DateTime | null {
    return this.connection.value?.lastReferencedAt ?? null;
  }

  incRefCount(): void {
    this.connection.value?.incRefCount();
  }

  decRefCount(): void {
    this.connection.value?.decRefCount();
  }

  close(): Promise<void> {
    return this.activeConnection.close();
  }

  togglePaused(): void {
    this.activeConnection.togglePaused();
  }

  onError(handler: (status: GrpcStatusName) => void): () => void {
    let sub: (() => void) | null = null;
    watch(this.connection, () => {
      if (sub) sub();
      if (this.connection.value != null) sub = this.connection.value.onError(handler);
    });
    return () => {
      if (sub) sub();
    };
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

  get activeConnection(): ConnectionBase<K, T> {
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
// Connection maintenance
// NOTE :Performance: cache/store connections (results) locally for initial hydration?
//

const INACTIVE_CONNECTION_TIMEOUT = 30 * 1000; // 30 seconds

let connectionId = 0;
function newConnectionId(): number {
  return connectionId++;
}
const localConnection = new LocalGetConnection(
  { id: newConnectionId(), name: "local.space", live: true, options: {} },
  { scope: EMPTY_SCOPE, roots: [LOCAL_SPACE_PTR], descendantTypes: [NodeType.VIEW] },
  new ProxyNodeGraph({ graph: spaceGraphLocal, filter: DEFAULT_NODE_FILTER }),
  // we export it as read-only but it's actually writable
  spaceGraphLocal as ReadNodeGraph & WriteNodeGraph,
);
const _connections: Ref<ConnectionBase<any, any>[]> = shallowRef([localConnection]);
export const connections = pretendReadonly(_connections);
export const hasPendingConnections = computed(() => connections.value.some((c) => !c.isConnected.value));

export const supergraph = new NodeSuperGraph();
const connectionWatcherByConnection = new Map<any, () => void>();

/** Adds a new connection to the connection set */
function _addConnection(connection: ConnectionBase<any, any>): void {
  _connections.value = [..._connections.value, connection];
  const sub = watch(
    connection.result,
    (newResult, oldResult) => {
      if (oldResult != null && "graph" in oldResult) {
        supergraph.removeGraph(oldResult.graph);
      }
      if (newResult != null && "graph" in newResult) {
        supergraph.addGraph(newResult.graph);
      }
    },
    { immediate: true },
  );
  connectionWatcherByConnection.set(connection, sub);
}

/** Removes a connection from the connection set */
async function _removeConnection(connection: ConnectionBase<any, any>): Promise<void> {
  await connection.close();
  const connectionIdx = _connections.value.indexOf(connection);
  if (connectionIdx >= 0) _connections.value.splice(connectionIdx, 1);
  triggerRef(_connections);
  const sub = connectionWatcherByConnection.get(connection);
  if (sub) sub();
}

/** GC inactive (non-local) connections that have been idle for some time */
async function gcInactiveConnections() {
  const inactiveConnections = _connections.value.filter(
    (c) =>
      Object.getPrototypeOf(c) != LocalGetConnection.prototype &&
      c.referenceCount == 0 &&
      c.lastReferencedAt != null &&
      DateTime.now().diff(c.lastReferencedAt).milliseconds > INACTIVE_CONNECTION_TIMEOUT,
  );
  if (inactiveConnections.length > 0) {
    log.trace("graph.gcInactiveConnections", { count: inactiveConnections.length });
    await Promise.all(inactiveConnections.map((c) => _removeConnection(c)));
  }
}

// periodically clean up inactive connections
setInterval(gcInactiveConnections, INACTIVE_CONNECTION_TIMEOUT);

/** RC-=1. Connections without references are GCed after some time. */
function releaseConnection(connection: ConnectionBase<any, any>): void {
  connection.decRefCount();
}

type ConnectionMatchOptions<K extends GraphConnectionKind, T extends NodeType> = {
  predicate?: (c: ConnectionBase<K, T>) => boolean;
};

/** Finds an existing connection */
export function findExistingConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  params: ConnectionParamsMapping<T>[K],
  match?: ConnectionMatchOptions<K, T>,
): ConnectionBase<K, T> | null {
  const matchingConnections =
    _connections.value.filter((c) => c.kind == kind && c.includes(params) && match?.predicate?.(c) !== false) ?? null;
  if (matchingConnections.length == 0) return null;
  if (matchingConnections.length > 1) {
    // TODO :Broken: find the best connection match somehow :ConnectionMatching
  }
  return matchingConnections[0];
}

export function findExistingConnectionOrError<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  params: ConnectionParamsMapping<T>[K],
  match?: ConnectionMatchOptions<K, T>,
): ConnectionBase<K, T> {
  const connection = findExistingConnection(kind, params, match);
  if (connection == null)
    throw new Error(
      `no connection found for ${kind}:${JSON.stringify(params)} (available: ${_connections.value.map((c) => c.name).join(", ")})`,
    );
  return connection;
}

/** Finds an existing connection and acquires it (RC+=1) */
function acquireExistingConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  params: ConnectionParamsMapping<T>[K],
  match?: ConnectionMatchOptions<K, T>,
): ConnectionBase<K, T> | null {
  const connection = findExistingConnection(kind, params, match);
  if (connection != null) connection.incRefCount();
  return connection;
}

export async function clearConnections(): Promise<void> {
  await Promise.all(_connections.value.map((c) => c.close()));
  _connections.value = [localConnection];
}

type ConnectionMetadataIn = Pick<ConnectionMetadata, "name"> & Partial<ConnectionMetadata>;

/** Creates a new (remote) connection and immediately acquires it (RC+=1). */
async function acquireNewConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  metaIn: Pick<ConnectionMetadata, "name"> & Partial<ConnectionMetadata>,
  params: ConnectionParamsMapping<T>[K],
): Promise<ConnectionBase<K, T>> {
  const meta: ConnectionMetadata = { live: false, id: newConnectionId(), options: {}, ...metaIn };

  // create
  const scope = getScopeFromParams(params);
  const txBuffer = getTransactionBuffer(scope);
  let connection: ConnectionBase<K, T>;
  if (kind == "get") {
    const getParams = params as GetConnectionParams<T>;
    connection = new RemoteGetConnection<T>(meta, getParams, txBuffer) as any as ConnectionBase<K, T>;
  } else if (kind == "search") {
    const searchParams = params as SearchConnectionParams<T>;
    connection = new RemoteSearchConnection<T>(meta, searchParams, txBuffer) as any as ConnectionBase<K, T>;
  } else {
    throw new Error(`unsupported connection kind: ${kind}`);
  }
  connection = markRaw(connection); // ensure it's never proxied
  _addConnection(connection);
  connection.incRefCount();

  // connect
  await connection.connect(meta.options);

  return connection;
}

/** Container for providing the results of a Get connection to an inner component */
export type PreparedGetConnection<T extends NodeType = NodeType> = {
  connection: Connection<"get", T>;
  graph: ReadNodeGraph;
};

/** Container for providing the results of a Search connection to an inner component */
export type PreparedSearchConnection<T extends NodeType = NodeType> = {
  connection: Connection<"search", T>;
  graph: ReadNodeGraph;
};

export type PreparedNodeConnection = PreparedGetConnection | PreparedSearchConnection;

/** Gets or acquires a connection given the params, maintaining reference counts and such. */
export function useConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<ConnectionParamsMapping<T>[K]>,
  match?: ConnectionMatchOptions<K, T>,
): {
  connection: Ref<ConnectionBase<K, T> | null>;
  isConnecting: Ref<boolean>;
  isConnected: Ref<boolean>;
  isStale: Ref<boolean>;
} {
  const connection: ShallowRef<ConnectionBase<K, T> | null> = shallowRef(null);
  const isConnecting = computed(() => connection.value?.isConnecting.value ?? false);
  const isConnected = ref(false);
  const isStale = ref(false);
  const paramsRef = toRef(params) as Ref<ConnectionParamsMapping<T>[K]>;

  // acquire existing or create new connection
  watch(
    toValueRef(paramsRef),
    async () => {
      if (paramsRef.value.isEnabled === false) {
        return; // disabled
      }
      const oldConnection = connection.value;
      let newConnection = findExistingConnection(kind, paramsRef.value, match);
      if (oldConnection != null && oldConnection === newConnection) {
        return; // no change
      }

      // acquire new connection
      if (oldConnection) {
        isStale.value = true;
      }
      if (newConnection) {
        newConnection.incRefCount();
      } else if (!newConnection) {
        newConnection = await acquireNewConnection(kind, metaIn, paramsRef.value);
      }
      connection.value = newConnection;
      if (oldConnection) {
        releaseConnection(oldConnection);
      }
      isStale.value = false;
      isConnected.value = true;
    },
    { immediate: true },
  );

  return { connection, isConnecting, isConnected, isStale };
}

/** The graph of a node connection overlaid with its local overlay */
function useConnectionGraphWithOverlay<T extends NodeType>(
  connection: Ref<ConnectionBase<"get" | "search", T> | null>,
): ReadNodeGraph {
  const graph = new LayerNodeGraph({ filter: DEFAULT_NODE_FILTER });
  watch(
    () => connection.value?.result.value,
    () => {
      if (connection.value?.result.value == null) {
        graph.layers.value = [];
      } else if (connection.value?.result.value.overlay == null) {
        graph.layers.value = [connection.value.result.value.graph];
      } else {
        graph.layers.value = [connection.value.result.value.graph, connection.value.result.value.overlay];
      }
    },
    { immediate: true },
  );
  return markRaw(graph); // ensure it's never proxied
}

/**
 * Gets the current connection for the given scope. Does not acquire any new connections.
 * NOTE: for performance the graph/connection proxies are 'lazy' (just regular refs, so they get batch-processed per tick).
 *  That means changing 'node' will change connection/graph only on the next tick.
 */
export function useExistingConnection<T extends NodeType = any>(
  node: MaybeRef<NodeReferenceData | TypedNodeReferenceData<any> | null | undefined>,
  options?: {
    isEnabled?: Ref<boolean>;
    isRequired?: boolean;
    match?: ConnectionMatchOptions<"get", T>;
  },
): {
  graph: ReadNodeGraph;
  connection: Connection<"get", T>;
} {
  // NOTE :Architecture: the graphs and the current bench/pkg/space pointers are not atomically updated,
  //  so sometimes it can happen that we need a new connection but the new graph isn't loaded yet.
  //  For those cases it's useful to just default to not required and keeping previous connections.

  const nodeRef = toValueRef(toRef(node)) as Ref<NodeReferenceData>;
  const connection: ShallowRef<ConnectionBase<"get", T> | null> = shallowRef(null);
  const graph = useConnectionGraphWithOverlay(connection);

  // route to the appropriate connection
  const refreshConnection = () => {
    const oldConnection = connection.value;
    let newConnection = null;
    if (connection.value) releaseConnection(connection.value);

    if (nodeRef.value != null && options?.isEnabled?.value !== false) {
      if (nodeRef.value?.id == null) {
        throw new Error(`missing id for connection: ${describeNode(nodeRef.value)}`);
      }
      newConnection = acquireExistingConnection(
        "get",
        { scope: PACKAGE_SCOPE.value, roots: [nodeRef.value as TypedNodeReferenceData<T>] },
        options?.match,
      );
      if (newConnection == null && options?.isRequired) {
        if (connection.value != null) {
          return; // ignore, we already have a connection
        } else {
          throw new Error(
            `missing connection for ${describeNode(nodeRef.value)} (available: ${_connections.value.map((c) => c.name).join(", ") ?? "<none>"})`,
          );
        }
      }
    }
    if (newConnection !== oldConnection) {
      connection.value = newConnection as ConnectionBase<"get", T> | null;
    }
  };
  watch(() => [nodeRef.value, () => options?.isEnabled?.value], refreshConnection, { immediate: true });

  // NOTE: useExistingConnection is usually used where a connection must exist (inside View components).
  //  Otherwise if we don't have a connection we need to check *every* new connection until we get a match.
  let stopGlobalWatch = null as (() => void) | null;
  watch(
    connection,
    () => {
      stopGlobalWatch?.();
      if (!connection.value) stopGlobalWatch = watch(_connections, refreshConnection);
    },
    { immediate: true, flush: "sync" },
  );

  // release on unmount
  tryOnBeforeUnmount(() => {
    if (connection.value) releaseConnection(connection.value);
  });

  return { graph, connection: new ProxyConnection(connection) };
}

/**
 * Gets the given nodes from the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that edits for the given nodes are watched.
 */
export function useGetConnection<T extends NodeType>(
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<GetConnectionParams<T>>,
): GetConnectionResult<T> & {
  connection: Connection<"get", T>;
  isConnecting: Ref<boolean>;
  isConnected: Ref<boolean>;
  isStale: Ref<boolean>;
} {
  const paramsRef = toRef(params) as Ref<GetConnectionParams<T>>;
  const { connection, isConnecting, isConnected, isStale } = useConnection<"get", T>("get", metaIn, paramsRef);

  // map results
  // NOTE :Cleanup: mapping connection results is a deep ref chain?
  const graph = useConnectionGraphWithOverlay(connection);
  const roots: Ref<NodeTypeMapping[T][]> = computed(() => connection.value?.result.value?.roots?.value ?? []);

  // no overlay because already overlaid
  return {
    graph,
    overlay: null,
    connection: new ProxyConnection(connection),
    roots,
    epoch: computed(() => connection.value?.epoch ?? -1n),
    isConnecting,
    isConnected,
    isStale,
  };
}

/**
 * Gets the given node from the relevant subgraph, fetching/caching automatically.
 * Wrapper around connections that just gives you the node directly.
 */
export function useNode<T extends NodeType>(paramsIn: {
  name: string;
  live?: boolean;
  scope?: MaybeRef<GraphScopeData>;
  nodePtr: MaybeRef<NodeReferenceData | TypedNodeReferenceData<T> | null | undefined>;
  ancestorTypes?: MaybeRef<NodeType[]>;
  descendantTypes?: MaybeRef<NodeType[]>;
  isOptional?: boolean;
  isEnabled?: Ref<boolean | undefined>;
}): {
  connection: Connection<"get" | "search", T>;
  node: Ref<NodeTypeMapping[T] | null>;
  isConnecting: Ref<boolean>;
  isConnected: Ref<boolean>;
  isStale: Ref<boolean>;
} {
  const { graph, connection, isConnecting, isConnected, isStale } = useGetConnection<T>(
    { name: paramsIn.name, live: paramsIn.live === undefined ? true : paramsIn.live },
    computed(
      () =>
        ({
          scope: toValue(paramsIn.scope) ?? PACKAGE_SCOPE.value,
          roots: [toValue(paramsIn.nodePtr)!],
          isOptional: paramsIn.isOptional,
          isEnabled: toValue(paramsIn.nodePtr) != null && toValue(paramsIn.isEnabled) !== false,
          ancestorTypes: toValue(paramsIn.ancestorTypes),
          descendantTypes: toValue(paramsIn.descendantTypes),
        }) as GetConnectionParams<T>,
    ),
  );
  const node = graph.getRef(paramsIn.nodePtr) as Ref<NodeTypeMapping[T] | null>;
  return { connection, node, isConnecting, isConnected, isStale };
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * If live, will also ensure that 1) edits for the result nodes are watched and 2) the search itself is watched.
 * TODO :Incomplete!: paginate (across) search connections
 */
export function useSearchConnection<T extends NodeType>(
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<SearchConnectionParams<T>>,
): SearchConnectionResult<T> & {
  roots: Ref<NodeTypeMapping[T][]>;
  connection: Connection<"search", T>;
  isConnecting: Ref<boolean>;
  isConnected: Ref<boolean>;
  isStale: Ref<boolean>;
} {
  const paramsRef = toRef(params) as Ref<SearchConnectionParams<T>>;
  const { connection, isConnecting, isConnected, isStale } = useConnection<"search", T>("search", metaIn, paramsRef);

  // map results
  const graph = useConnectionGraphWithOverlay(connection);
  const rootsPtr: Ref<TypedNodeReferenceData<T>[]> = computed(
    () => connection.value?.result?.value?.rootsPtr?.value ?? [],
  );
  const roots = graph.getManyRef(rootsPtr);
  const page: Ref<PageInfo> = computed(
    () => connection.value?.result?.value?.page?.value ?? ({ roots: [], cursors: [], size: 0 } as PageInfo),
  );

  // no overlay because already overlaid
  return {
    graph,
    overlay: null,
    connection: new ProxyConnection(connection),
    rootsPtr,
    roots,
    epoch: computed(() => connection.value?.epoch ?? -1n),
    page,
    isConnecting,
    isConnected,
    isStale,
  };
}

/**
 * Aggregates nodes of the given type in the relevant subgraph, fetching/caching automatically.
 * NOTE :Incomplete: live aggregation
 */
export function useAggregateConnection(
  params: MaybeRef<AggregateConnectionParams>,
): AggregateConnectionResult & { connection: ConnectionBase<"aggregate", NodeType> } {
  throw new Error("aggregate not yet implemented");
}

/**
 * Checks whether the node exists in the backend graph or only in our optimistic overlay.
 * (useful when waiting to perform operations until a node has been committed, but feels hacky.. :SearchWithMissingBlock)
 **/
export function useNodeIsCommitted<T extends NodeType>(
  connection: Connection<"get" | "search", T>,
  node: MaybeRef<NodeReferenceData | TypedNodeReferenceData<T> | null | undefined>,
): Ref<boolean> {
  const nodePtrRef = toValueRef(toRef(node)) as Ref<NodeReferenceData>;
  const checkIsCommitted = () =>
    nodePtrRef.value != null && (connection.result?.value?.graph.has(nodePtrRef.value) ?? false);
  const isCommitted: Ref<boolean> = ref(checkIsCommitted());

  if (!isCommitted.value) {
    // NOTE :Cleanup: this is a bad way to wait for the node to be committed, but we only use this
    //  in one place and will hopefully have a better way soon :SearchWithMissingBlock
    // watch node ref until node exists in graph
    const check = setInterval(() => {
      if (checkIsCommitted()) {
        isCommitted.value = true;
        clearInterval(check);
      }
    }, 500);
  }

  return isCommitted;
}

//
// Connection keep-alive for active streaming connections
//

const _activeRemoteConnections = computed(() =>
  Object.values(connections.value).filter((c) => c.isConnected.value && c.isLive && !c.name.startsWith("local.")),
);
const _activeRemoteBenchIds = computed(() => {
  const benchIds = new Set<string>();
  _activeRemoteConnections.value.map((c) => benchIds.add(c.params.scope.benchId));
  return [...benchIds];
});

export async function sendRemoteKeepAlives() {
  const healthChecks = [];
  for (const benchId of _activeRemoteBenchIds.value) {
    const transport = getGraphTransport({ benchId });
    if (transport == null) continue;
    const healthClient = new HealthClient(transport);
    healthChecks.push(
      healthClient.check({ service: benchId != null ? "symbolx.bench.Host" : "symbolx.bench.Supervisor" }),
    );
  }
  await Promise.all(healthChecks);
}
