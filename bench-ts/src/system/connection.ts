import { setAutoloader, setSupergraph } from "@/globals";
import { makeAndConditional, makeExpression } from "@/language/core/expression";
import {
  DEFAULT_NODE_FILTER,
  LayerNodeGraph,
  NodeGraph,
  NodeSuperGraph,
  ProxyNodeGraph,
  type ReadNodeGraph,
  type WriteNodeGraph,
} from "@/language/core/graph";
import {
  ImmediateTransactionBuffer,
  editGraph,
  getTransactionBuffer,
  newBufferId,
  type Transaction,
  type TransactionBuffer,
} from "@/language/core/transaction";
import {
  HUMANIZED_OPERATION_STATUS,
  getGraphClient,
  getGraphTransport,
  type GrpcStatusName,
  type OperationMetadata,
} from "@/proto/services";
import {
  AggregationResultData,
  EditData,
  EmptyProperty,
  ExpressionData,
  ExpressionType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  SelectOptionsData,
  Timestamp,
  type GraphScopeData,
  type NodeTypeMapping,
} from "@/proto/wire";
import { HealthClient } from "@/proto/wire/proto/health.client";
import {
  EMPTY_SCOPE,
  deepContentEquals,
  makeDefaultObject,
  makeScope,
  propertyReference,
  unwrapSomeNode,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { NodeAutoloader } from "@/system/autoload";
import { LOCAL_SPACE_PTR, packagePtr, spaceGraphLocal } from "@/system/client";
import { toaster } from "@/ui/toast";
import { AsyncEvent, assertNever } from "@/utils/functools";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { immediateStopWatch, pretendReadonly, toValueRef } from "@/utils/ref";
import { captureException } from "@/utils/telemetry";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { useNetwork, whenever } from "@vueuse/core";
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
  /** Whether this is read-only (no edits) */
  isReadOnly?: boolean;
};

// get connection
export type GetConnectionParams<T extends NodeType> = {
  isEnabled?: boolean;
  scope: GraphScopeData;
  roots: (Omit<NodeReferenceData, "type"> & { nodeType: T })[];
  baseTypePtr?: NodeReferenceData;
  isOptional?: boolean;
  ancestorTypes?: NodeType[];
  descendantTypes?: NodeType[];
  select?: Partial<SelectOptionsData>;
  includeRemoved?: boolean;
  noMemory?: boolean;
};
export type GetConnectionResult<T extends NodeType> = {
  graphRaw: ReadNodeGraph; // graph without overlay (if different)
  graphOverlay: ReadNodeGraph | null; // graph overlay (only)
  graphComposite: ReadNodeGraph; // composite graph (raw + overlay)
  graph: ReadNodeGraph;
  roots: Ref<NodeTypeMapping[T][]>;
  epoch: Ref<bigint>;
};

// search connection
export type SearchConnectionParams<T extends NodeType> = {
  isEnabled?: boolean;
  scope: GraphScopeData;
  nodeType: T;
  baseTypePtr?: NodeReferenceData;
  filter?: ExpressionData;
  sort?: ExpressionData[];
  ancestorTypes?: NodeType[];
  descendantTypes?: NodeType[];
  first?: number;
  count?: boolean;
  select?: Partial<SelectOptionsData>;
};
export type SearchConnectionResult<T extends NodeType> = {
  graphRaw: ReadNodeGraph; // graph without overlay (if different)
  graphOverlay: ReadNodeGraph | null; // graph overlay (only)
  graphComposite: ReadNodeGraph; // composite graph (raw + overlay)
  graph: ReadNodeGraph;
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
 * Create an overlay graph for a specific Connection from a TransactionBuffer.
 */
function makeConnectionOverlayGraph(
  base: NodeGraph,
  connection: Connection<any, any>,
): { graph: NodeGraph; sub: () => void } {
  const overlay = new NodeGraph({ scope: base.scope, nodeTypes: base.nodeTypes, isOverlayOf: base });

  // add current buffered edits
  {
    const currentBuffered = connection.txBuffer.getBufferByConnection();
    const currentEdits = [...(currentBuffered[-1] ?? []), ...(currentBuffered[connection.meta.id] ?? [])];
    editGraph(overlay, currentEdits, { base: base });
  }

  // susbcribe to buffer changes
  const sub = connection.txBuffer.subscribeBuffer((event) => {
    if (event.type == "reset") {
      overlay.clear();
    }
    let edits: EditData[];
    if (event.meta.connectionId == connection.meta.id) {
      // our connection, take all edits
      edits = event.bufferedEdits.filter((e) => base.nodeTypes.has(e.nodePtr?.nodeType!));
    } else if (event.meta.connectionId == null) {
      // general connection, take any edits that match our node types
      edits = event.bufferedEdits.filter((e) => base.nodeTypes.has(e.nodePtr?.nodeType!));
    } else {
      // other connection, take any edits that concern our own nodes
      edits = event.bufferedEdits.filter((e) => base.has(e.nodePtr!));
    }
    if (edits.length > 0) {
      // NOTE :Cleanup: we used to have ignoreMissing: event.meta.connectionId != connection.meta.id :RichGraph
      //  (but that doesn't totally work since we now have node types that are sometimes in the loaded graph, sometimes not
      //   e.g., we have Bench.memberships loaded but not Thread.memberships since Threads are unloaded, so those Memberships may be missing)
      editGraph(overlay, edits, { base: base, ignoreMissing: true });
    }
  });

  return { graph: overlay, sub };
}

/**
 * Apply edits from a buffer to a specific Connection's base graph (filtering as needed).
 */
function applyBufferCommit(
  connection: Connection<any, any>,
  graph: NodeGraph,
  commit: {
    edits: EditData[];
    cascadedEdits: EditData[];
    connectionIdByEditId: Record<string, number>;
  },
): void {
  const edits: EditData[] = [];
  for (const edit of commit.edits) {
    if (commit.connectionIdByEditId[edit.id] == connection.meta.id) {
      edits.push(edit);
    } else if (graph.has(edit.nodePtr!)) {
      edits.push(edit);
    }
  }
  if (edits.length > 0) {
    editGraph(graph, edits);
  }
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
  waitUntil(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void>;
};

type ConnectionInternalResult = { subs?: (() => void)[] };

export abstract class ConnectionBase<K extends GraphConnectionKind, T extends NodeType> {
  abstract readonly kind: K;

  readonly meta: ConnectionMetadata;
  readonly params: ConnectionParamsMapping<T>[K];
  readonly result: ShallowRef<(ConnectionResultMapping<T>[K] & ConnectionInternalResult) | null> = shallowRef(null);
  readonly txBuffer: TransactionBuffer;
  readonly nodeTypes: Set<NodeType>;
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
    this.nodeTypes = new Set(getNodeTypesFromParams(params));
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
    log.trace(`graph.${this.kind}.close`, { name: this.meta.name, id: this.id });
    this.abortController?.abort();
  }

  togglePaused(): void {
    this.isPaused.value = !this.isPaused.value;
    log.trace(`graph.${this.kind}.togglePaused`, { name: this.meta.name, id: this.id, paused: this.isPaused.value });
    toaster.debug({
      title: this.isPaused.value ? "Connection paused" : "Connection resumed",
      text: `'${this.kind}:${this.meta.name}' is ${this.isPaused.value ? "disconnected" : "reconnected"}.`,
      override: `connection.togglePaused:${this.meta.id}`,
    });
  }

  get operationMeta(): OperationMetadata {
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

  async waitUntil(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void> {
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
      captureException(error);
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
          title: `Disconnected`,
          text: `Connection failed: ${IS_DEV ? error.message : HUMANIZED_OPERATION_STATUS[(error as RpcError).code]}`,
          override: `connection:${this.meta.id}`,
          summarize: {
            key: "connection.error",
            info: [{ op, error: error as RpcError }],
            text: (infos) => {
              // distinct errors
              const errors = new Set(
                infos
                  .map((info) => HUMANIZED_OPERATION_STATUS[info.error.code])
                  .filter((e) => e != null && e.length > 0),
              );
              const errorsText = [...errors].join(", ") ?? "Unknown error";
              return `Connection failed: ${errorsText}`;
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
          log.trace(`graph.${this.kind}.connect`, this.meta.name, this.params);
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
            log.trace(`graph.${this.kind}`, this.meta.name, this.params, newResult);
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
            toaster.success({
              title: "Reconnected",
              text: `Connection restored.`,
              summarize: {
                key: "connection.reconnected",
                info: [{ op: `${this.kind}:${this.meta.name}` }],
              },
              override: `connection:${this.meta.id}`,
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
    const graph = new NodeGraph({ scope, nodeTypes: new Set(nodeTypes) });
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
        baseTypePtr: params.baseTypePtr,
        ancestorTypes: params.ancestorTypes ?? [],
        descendantTypes: params.descendantTypes ?? [],
        select: select,
        isOptional: params.isOptional,
        includeRemoved: params.includeRemoved,
        noMemory: params.noMemory,
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
        if (rep.isKeepalive) return; // ignore keepalives
        epoch.value = rep.epoch;
        // apply edits from stream
        // NOTE :Robustness: RemoteGetConnection ignoreMissing is wonky :RichGraph
        //  (seems to be missing an Agent node in a Thread after we delete and restore the containing Thread..?)
        editGraph(graph, [...rep.edits, ...rep.cascadedEdits], { ignoreMissing: true });
        this.txBuffer.onAccepted(rep.edits, rep.cascadedEdits);
      });
      editStream.responses.onError(onError);
      editStream.responses.onComplete(() => onError(new Error("edit stream closed")));
    } else {
      this.txBuffer.subscribeAccepted(({ edits, cascadedEdits, connectionIdByEditId }) => {
        // apply edits from any accepted buffers (counterpart to overlay)
        applyBufferCommit(this, graph, { edits, cascadedEdits, connectionIdByEditId });
      });
    }

    const { graph: overlay, sub } = makeConnectionOverlayGraph(graph, this);
    subs.push(sub);
    const graphComposite = new LayerNodeGraph({ layers: [graph, overlay], filter: DEFAULT_NODE_FILTER });
    return {
      graph,
      graphComposite,
      graphRaw: graph,
      graphOverlay: overlay,
      roots: graph.getManyRef(params.roots),
      epoch,
      subs,
    };
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
    const graph = new NodeGraph({ scope, nodeTypes: new Set(nodeTypes) });
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
        baseTypePtr: params.baseTypePtr,
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
        if (rep.isKeepalive) return; // ignore keepalives
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
        this.txBuffer.onAccepted(rep.edits, rep.cascadedEdits);
      });
      editStream.responses.onError(onError);
      editStream.responses.onComplete(() => onError(new Error("edit stream closed")));
    } else {
      this.txBuffer.subscribeAccepted(({ edits, cascadedEdits, connectionIdByEditId }) => {
        // apply edits from any accepted buffers (counterpart to overlay)
        applyBufferCommit(this, graph, { edits, cascadedEdits, connectionIdByEditId });
      });
    }

    const { graph: graphOverlay, sub } = makeConnectionOverlayGraph(graph, this);
    subs.push(sub);
    const graphComposite = new LayerNodeGraph({ layers: [graph, graphOverlay], filter: DEFAULT_NODE_FILTER });
    return { graph, graphComposite, graphRaw: graph, graphOverlay: graphOverlay, rootsPtr, page, subs, epoch };
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
      graphRaw: this.graph,
      graphOverlay: null,
      graphComposite: this.graph,
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
    return {
      graph: this.graph,
      graphRaw: this.graph,
      graphComposite: this.graph,
      graphOverlay: null,
      roots: this.graph.getManyRef(params.roots),
      epoch: ref(-1n),
    };
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

  async waitUntil(predicate: (result: ConnectionResultMapping<T>[K] | null) => boolean): Promise<void> {
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
// NOTE :Performance :UX: cache/store connections (results) locally for initial hydration?
//

const CONNECTION_AUTOLOAD_INTERVAL = 100;
const CONNECTION_INACTIVE_TIMEOUT = 30 * 1000; // 30 seconds

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
export const supergraph = new NodeSuperGraph(connections);
setSupergraph(supergraph);
export const autoloader = new NodeAutoloader(supergraph);
setAutoloader(autoloader);
supergraph.subscribeEvent((e, key, callback) =>
  autoloader.onEvent(e, { ...(key as NodeReferenceData), metatype: ObjectType.NODE_REFERENCE }, callback),
);

/** Adds a new connection to the connection set */
function _addConnection(connection: ConnectionBase<any, any>): void {
  _connections.value = [..._connections.value, connection];
}

/** Removes a connection from the connection set */
async function _removeConnection(connection: ConnectionBase<any, any>): Promise<void> {
  await connection.close();
  const connectionIdx = _connections.value.indexOf(connection);
  if (connectionIdx >= 0) _connections.value.splice(connectionIdx, 1);
  triggerRef(_connections);
}

/** GC inactive (non-local) connections that have been idle for some time */
async function gcInactiveConnections() {
  const inactiveConnections = _connections.value.filter(
    (c) =>
      Object.getPrototypeOf(c) != LocalGetConnection.prototype &&
      c.referenceCount == 0 &&
      c.lastReferencedAt != null &&
      DateTime.now().diff(c.lastReferencedAt).milliseconds > CONNECTION_INACTIVE_TIMEOUT,
  );
  if (inactiveConnections.length > 0) {
    log.trace("graph.gcInactiveConnections", { count: inactiveConnections.length });
    await Promise.all(inactiveConnections.map((c) => _removeConnection(c)));
  }
}

// periodically load missing nodes
setInterval(autoloader.loadAll.bind(autoloader), CONNECTION_AUTOLOAD_INTERVAL);
// TODO :Performance :Robustness: periodically clean up inactive autoloads / connections :RichGraph
//  (but we'll have a new query system for :RichGraph soon so this is hopefully moot)
// periodically clean up inactive connections
// setInterval(() => {
//   autoloader.gc();
//   gcInactiveConnections();
// }, CONNECTION_INACTIVE_TIMEOUT / 10);

type ConnectionMatchOptions<K extends GraphConnectionKind, T extends NodeType> = {
  predicate?: (c: ConnectionBase<K, T>) => boolean;
};

/** Finds an existing connection */
export function findExistingConnection<K extends GraphConnectionKind, T extends NodeType>(
  kinds: K[],
  params: ConnectionParamsMapping<T>[K],
  match?: ConnectionMatchOptions<K, T>,
  exclude?: ConnectionBase<K, T>,
): ConnectionBase<K, T> | null {
  const matchingConnections =
    _connections.value.filter(
      (c) =>
        c !== exclude &&
        kinds.includes(c.kind) &&
        deepContentEquals(c.params, params) &&
        match?.predicate?.(c) !== false,
    ) ?? null;
  if (matchingConnections.length == 0) {
    return null;
  }
  if (matchingConnections.length > 1) {
    // NOTE: find the best connection match somehow :RichGraph?
    // pick newest connection
    matchingConnections.sort((a, b) => b.createdAt.diff(a.createdAt).milliseconds);
  }
  return matchingConnections[0];
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

/** Acquire a (remote) connection (new or existing) */
export async function acquireConnection<K extends GraphConnectionKind, T extends NodeType>(
  kind: K,
  metaIn: ConnectionMetadataIn,
  params: ConnectionParamsMapping<T>[K],
  match?: ConnectionMatchOptions<K, T>,
  exclude?: ConnectionBase<K, T>,
): Promise<ConnectionBase<K, T>> {
  const connection = findExistingConnection([kind], params, match, exclude);
  if (connection != null) {
    connection.incRefCount();
    return connection;
  } else {
    return await acquireNewConnection(kind, metaIn, params);
  }
}

/** RC-=1. Connections without references are GCed after some time. */
export function releaseConnection(connection: ConnectionBase<any, any>): void {
  connection.decRefCount();
}

/** Drop a Connection immediately (ignoring reference counts). */
export function dropConnection(connection: ConnectionBase<any, any>): void {
  _removeConnection(connection);
}

/** Container for providing the results of a Get connection to an inner component */
export type PreparedNodeConnection<T extends NodeType = NodeType> = {
  connection: Connection<"get" | "search", T>;
  graph: ReadNodeGraph;
};

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
      let newConnection = findExistingConnection([kind], paramsRef.value, match);
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
function useConnectionGraphComposite<T extends NodeType>(
  connection: Ref<ConnectionBase<"get" | "search", T> | null>,
): ReadNodeGraph {
  // NOTE :Cleanup: the distinction between graphRaw/graphOverlay/graphComposite and this thing here is confusing?
  const graph = new LayerNodeGraph({ filter: DEFAULT_NODE_FILTER });
  watch(
    () => connection.value?.result.value,
    () => {
      if (connection.value?.result.value == null) {
        graph.layers.value = [];
      } else if (connection.value?.result.value.graphOverlay == null) {
        graph.layers.value = [connection.value.result.value.graph];
      } else {
        graph.layers.value = [connection.value.result.value.graph, connection.value.result.value.graphOverlay];
      }
    },
    { immediate: true },
  );
  return markRaw(graph); // ensure it's never proxied
}

/** The graph of a node connection without its overlay */
function useConnectionGraphRaw<T extends NodeType>(
  connection: Ref<ConnectionBase<"get" | "search", T> | null>,
): ReadNodeGraph {
  const graph = new ProxyNodeGraph({ filter: DEFAULT_NODE_FILTER });
  watch(
    () => connection.value?.result.value,
    () => {
      graph.graph = connection.value?.result.value?.graph ?? null;
    },
    { immediate: true },
  );
  return markRaw(graph); // ensure it's never proxied
}

/**
 * Gets any current connection for the given scope.
 * NOTE: for performance the graph/connection proxies are 'lazy' (just regular refs, so they get batch-processed per tick).
 *  That means changing 'node' will change connection/graph only on the next tick.
 * NOTE :Architecture: the graphs and the current bench/pkg/space pointers are not atomically updated,
 * NOTE :Performance: don't use separate overlay graphs for every useExistingConnection?
 *  so sometimes it can happen that we need a new connection but the new graph isn't loaded yet.
 *  For those cases it's useful to just default to not required and keeping previous connections.
 */
export function useAutoConnection<T extends NodeType = any>(
  node: MaybeRef<NodeReferenceData | TypedNodeReferenceData<any> | null | undefined>,
): {
  graph: ReadNodeGraph;
  graphRaw: ReadNodeGraph;
  connection: Connection<"get" | "search", T>;
} {
  const nodeRef = toValueRef(toRef(node)) as Ref<NodeReferenceData>;
  // restrict to get connections because search connections don't have descendants :BadSearchConnection
  const { connection } = supergraph.getLinkRef(nodeRef, { excludeSearch: true });
  const graph = useConnectionGraphComposite(connection);
  const graphRaw = useConnectionGraphRaw(connection);
  return { graph, graphRaw, connection: new ProxyConnection(connection) };
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
  const graph = useConnectionGraphComposite(connection);
  const graphRaw: ReadNodeGraph = useConnectionGraphRaw(connection);
  const roots: Ref<NodeTypeMapping[T][]> = computed(() => connection.value?.result.value?.roots?.value ?? []);

  // no overlay because already overlaid
  return {
    graph,
    graphRaw: graphRaw,
    graphOverlay: null,
    graphComposite: graph,
    connection: new ProxyConnection(connection),
    roots,
    epoch: computed(() => connection.value?.epoch ?? -1n),
    isConnecting,
    isConnected,
    isStale,
  };
}

/**
 * Searches for nodes of the given type in the relevant subgraph, fetching/caching automatically.
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
  const graph = useConnectionGraphComposite(connection);
  const graphRaw = useConnectionGraphRaw(connection);
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
    graphRaw: graphRaw,
    graphOverlay: null,
    graphComposite: graph,
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
 * Searches for nodes over two 'chunks' (pages) of results at a time for smooth scrolling (top and bottom).
 * We assume sort by one, going from bottom to top (latest at the bottom, earliest at the top).
 * */
export function useInfiniteSearchConnection<T extends NodeType>(
  metaIn: ConnectionMetadataIn,
  params: MaybeRef<Omit<SearchConnectionParams<T>, "first" | "sort" | "isEnabled">>,
  options: {
    nodeType: T;
    chunkSize: number;
    isEnabled: Ref<boolean>;
    onAdded?: () => void;
  },
) {
  type Side = "main" | "other";
  const topSide: Ref<Side> = ref("main");
  const bottomSide: Ref<Side> = ref("main");
  const cursorStack: Ref<(Timestamp | null)[]> = ref([]);
  const mainActive: Ref<boolean> = ref(true);
  const mainCursor: Ref<Timestamp | null> = ref(null);
  const otherActive: Ref<boolean> = ref(false);
  const otherCursor: Ref<Timestamp | null> = ref(null);
  const sort = makeExpression({
    type: ExpressionType.DESCENDING,
    propertyPtr: propertyReference(options.nodeType, EmptyProperty.createdAt),
  });

  function makeSearchParams(isActive: boolean, cursor: Timestamp | null) {
    const paramsValue = toValue(params);
    const combinedParams: SearchConnectionParams<T> = {
      ...paramsValue,
      first: options.chunkSize,
      isEnabled: options.isEnabled.value && isActive,
      sort: [sort],
    };
    if (cursor != null) {
      const cursorFilter = makeExpression({
        type: ExpressionType.LESS_THAN,
        value: cursor,
        propertyPtr: propertyReference(options.nodeType, EmptyProperty.createdAt),
      });
      combinedParams.filter =
        combinedParams.filter != null ? makeAndConditional([combinedParams.filter, cursorFilter]) : cursorFilter;
    }
    return combinedParams;
  }
  const main = useSearchConnection(
    metaIn,
    computed(() => makeSearchParams(mainActive.value, mainCursor.value)),
  );
  const other = useSearchConnection(
    metaIn,
    computed(() => makeSearchParams(otherActive.value, otherCursor.value)),
  );

  function getTop() {
    if (topSide.value == "main") {
      return main;
    } else if (topSide.value == "other") {
      return other;
    } else {
      assertNever(topSide.value);
    }
  }

  function getBottom() {
    if (bottomSide.value == "main") {
      return main;
    } else if (bottomSide.value == "other") {
      return other;
    } else {
      assertNever(bottomSide.value);
    }
  }

  // deduplicate and sort roots from both chunks
  const roots: Ref<NodeTypeMapping[T][]> = computed(() => {
    const rootsById: Record<string, NodeTypeMapping[T]> = {};
    for (const root of main.roots.value) {
      rootsById[root.id] = root;
    }
    for (const root of other.roots.value) {
      rootsById[root.id] = root;
    }
    return Object.values(rootsById).sort((a, b) => {
      if (a.createdAt == null || b.createdAt == null) return 0;
      else if (a.createdAt.seconds == b.createdAt.seconds) return a.createdAt.nanos - b.createdAt.nanos;
      else return Number(a.createdAt.seconds - b.createdAt.seconds);
    });
  });

  const isAtStart: Ref<boolean> = computed(() => {
    const top = getTop();
    return top.page.value.total! <= top.page.value.size;
  });
  const isAtEnd: Ref<boolean> = computed(() => {
    return cursorStack.value.length <= 1; // two chunks, so if we have one cursor, one of them is at the end
  });

  function go(direction: "up" | "down"): boolean {
    // can't go if we're loading
    if ((mainActive.value && !main.isConnected.value) || (otherActive.value && !other.isConnected.value)) {
      return false;
    }

    if (direction == "up") {
      // move bottom to top, swap sides
      if (isAtStart.value) {
        return false;
      }
      const nextCursor = roots.value[0].createdAt!;
      if (topSide.value == "main") {
        cursorStack.value.push(otherCursor.value);
        otherCursor.value = nextCursor;
        otherActive.value = true;
        topSide.value = "other";
        bottomSide.value = "main";
      } else if (topSide.value == "other") {
        cursorStack.value.push(mainCursor.value);
        mainCursor.value = nextCursor;
        mainActive.value = true;
        topSide.value = "main";
        bottomSide.value = "other";
      }
    } else if (direction == "down") {
      if (isAtEnd.value) {
        return false;
      }
      const nextCursor = cursorStack.value.pop() ?? null;
      if (bottomSide.value == "main") {
        otherCursor.value = nextCursor;
        otherActive.value = true;
        bottomSide.value = "other";
        topSide.value = "main";
      } else if (bottomSide.value == "other") {
        mainCursor.value = nextCursor;
        mainActive.value = true;
        bottomSide.value = "main";
        topSide.value = "other";
      }
    }
    return true;
  }

  return {
    roots,
    txFactory: () => main.connection.tx,
    isConnected: computed(() => main.isConnected.value),
    isAtStart,
    isAtEnd,
    topSide,
    bottomSide,
    main,
    mainActive,
    mainCursor,
    other,
    otherActive,
    otherCursor,
    cursorStack,
    go,
  };
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
      healthClient.check({ service: benchId != null ? "symbol.bench.Host" : "symbol.bench.Supervisor" }),
    );
  }
  await Promise.all(healthChecks);
}
