import {
  ResolveHostsResponse_HostInfo,
  GraphScopeData,
  HostClient,
  RpcMetadata,
  SupervisorClient,
  type IGraphIOClient,
} from "@/proto/wire";
import { CLIENT_TYPE, clientInfo, clientMeta } from "@/system/client";
import { toaster } from "@/ui/toast";
import { SUPERVISOR_URL } from "@/utils/globals";
import { log } from "@/utils/log";
import { formatDuration } from "@/utils/time";
import { GrpcStatusCode, GrpcWebFetchTransport, type GrpcWebOptions } from "@protobuf-ts/grpcweb-transport";
import {
  RpcError,
  type MethodInfo,
  type RpcOptions,
  type ServerStreamingCall,
  type UnaryCall,
} from "@protobuf-ts/runtime-rpc";
import { DateTime, Duration } from "luxon";
import { computed, shallowRef, triggerRef, type Ref } from "vue";

// NOTE :Robustness: ipv6 on MacOS sometimes causes ERR_NETWORK_CHANGED in chrome, breaking RPC streams.
//  A 'solution' is to disable ipv6, but obviously you have to do that manually as a user:
//   > networksetup -setv6off "Wi-Fi"
//  For now this appears to be a rare issue, only on localhost / development.
//  See https://forum.manjaro.org/t/err-network-changed-sometimes/135443
//  and https://superuser.com/questions/747735/regularly-getting-err-network-changed-errors-in-chrome

/** An operation is an RPC call which may be retried. */
export type Operation<I extends object, O extends object> = {
  transport: BenchGrpcWebTransport;
  id: number;
  name: string;
  method: MethodInfo<I, O>;
  options: RpcOptions;
  request: I;
  response?: O; // for unary
  error?: OperationError;
  numResponses?: number; // for streaming
  numRetries?: number;
  call: ServerStreamingCall<I, O> | UnaryCall<I, O>; // last successful call (if retried)
  startedAt: DateTime;
  updatedAt?: DateTime; // for streaming
  terminatedAt?: DateTime;
  duration?: Duration;

  isStreaming: boolean;
  get isPending(): boolean;
  abort?(): void;
};

export type OperationError = RpcError | Error;
export type GrpcStatusName = keyof typeof GrpcStatusCode;

type RetryOptions<T extends object> = {
  retryOn?: GrpcStatusName[];
  maxRetries?: number;
  maxTimeMs?: number;
  amendRetry?: (request: T, error: RpcError, numRetries: number) => T;
};
export type OperationMetadata<T extends object> = {
  connectionId?: number;
  operationName?: string;
  suppressErrors?: boolean;
  retry?: RetryOptions<T>;
};
export type OperationOptions = RpcOptions & OperationMetadata<any>;

export const HUMANIZED_OPERATION_STATUS: { [key: string]: string } = {
  INVALID_ARGUMENT: "Invalid request",
  OUT_OF_RANGE: "Invalid request",
  NOT_FOUND: "Not found",
  ALREADY_EXISTS: "Already exists",
  UNAUTHENTICATED: "Not authenticated",
  FAILED_PRECONDITION: "Cannot do this right now",
  PERMISSION_DENIED: "Not allowed",
  RESOURCE_EXHAUSTED: "Server overloaded",
  UNAVAILABLE: "System unavailable",
  NOT_IMPLEMENTED: "Not yet supported",
  INTERNAL: "Internal server error",
  CANCELLED: "Request cancelled",
  DEADLINE_EXCEEDED: "Request timed out",
};
export const HUMANIZED_OPERATION_MESSAGE: { [key: string]: string } = {
  UNAUTHENTICATED: "Please log in and try again",
  UNAVAILABLE: "System could not be reached",
};

export function humanizeError(error: OperationError): { title: string; text: string } {
  if (error instanceof RpcError) {
    const title = HUMANIZED_OPERATION_STATUS[error.code] ?? "Server error";
    const message = HUMANIZED_OPERATION_MESSAGE[error.code] ?? error.message;
    return { title, text: message };
  } else {
    return { title: "Error", text: error.message };
  }
}

const operationsTracker = {
  RECENT_OPERATION_BUFFER_SIZE: 1000,

  recentOps: [] as Operation<any, any>[],
  pendingOps: [] as Operation<any, any>[],
  numTotalOps: 0,

  get recent() {
    return this.recentOps;
  },

  get pending() {
    return this.pendingOps;
  },

  track<I extends object, O extends object>(opIn: Omit<Operation<I, O>, "id" | "name">) {
    const id = this.numTotalOps++;
    const uri = opIn.transport.uri;
    const serviceName = opIn.method.service.typeName.split(".").slice(2).join(".");
    const op = { ...opIn, id, name: `${serviceName}.${opIn.method.name}` };
    const rpcName = `rpc.${op.name}`;
    this.pendingOps.push(op);
    this.recentOps.push(op);
    if (this.recentOps.length > this.RECENT_OPERATION_BUFFER_SIZE) {
      this.recentOps.shift();
    }
    log.trace(rpcName, { id, uri, request: op.request });

    const remove = () => {
      const index = this.pendingOps.indexOf(op);
      if (index !== -1) this.pendingOps.splice(index, 1);
    };
    const terminate = () => {
      op.terminatedAt = DateTime.now();
      op.duration = op.terminatedAt.diff(op.startedAt, "seconds");
    };
    const onError = async (error: RpcError) => {
      const code = error.code;
      const meta = op.options as OperationMetadata<any>;
      if (!meta.suppressErrors) {
        log.error(rpcName, { id, uri, code, error, op });
        toaster.error(humanizeError(error));
      }
      if (code == "UNAUTHENTICATED") {
        const { onAuthenticationError } = await import("@/system/user"); // recursive import
        onAuthenticationError(error);
      }
    };

    // subscribe to call
    if ("responses" in op.call) {
      // streaming
      op.call.responses.onNext((r) => {
        op.updatedAt = DateTime.now();
        op.numResponses = (op.numResponses ?? 0) + 1;
        log.trace(`${rpcName}.update`, { id, uri, numResponses: op.numResponses, epoch: (r as any)?.epoch });
      });
      op.call.responses.onComplete(() => {
        terminate();
        log.trace(`${rpcName}.complete`, { id, uri, duration: formatDuration(op.duration!, { maxUnit: "ms" }) });
        remove();
      });
      op.call.responses.onError((error) => {
        op.error = error;
        terminate();
        onError(error as RpcError);
        remove();
      });
    } else {
      // unary
      op.call.response
        .then((output) => {
          op.response = output;
          terminate();
          log.trace(`${rpcName}.complete`, {
            id,
            uri,
            duration: formatDuration(op.duration!, { maxUnit: "ms" }),
            output,
          });
        })
        .catch((error) => {
          op.error = error;
          terminate();
          onError(error);
        })
        .finally(() => {
          remove();
        });
    }

    return op;
  },
};

export type BenchServerStreamingCall<I extends object, O extends object> = ServerStreamingCall<I, O> & {
  operation: Operation<I, O>;
};

export type BenchUnaryCall<I extends object, O extends object> = UnaryCall<I, O> & {
  operation: Operation<I, O>;
};

/**
 * Extend the standard grpc-web fetch clients with aut & instrumentation.
 */
class BenchGrpcWebTransport extends GrpcWebFetchTransport {
  uri: string;

  constructor(uri: string) {
    super({ baseUrl: `https://${uri}` });
    this.uri = uri;
  }

  mergeOptions(options?: Partial<RpcOptions> | undefined): RpcOptions {
    options = super.mergeOptions(options);
    options = { ...options, meta: { ...currentMetadataEncoded.value } };
    return options;
  }

  serverStreaming<I extends object, O extends object>(
    method: MethodInfo<I, O>,
    input: I,
    options: OperationOptions,
  ): BenchServerStreamingCall<I, O> {
    if (options.retry) throw new Error("serverStreaming does not support retry options");
    const call = super.serverStreaming(method, input, options) as BenchServerStreamingCall<I, O>;
    const op = operationsTracker.track({
      transport: this,
      method,
      request: input,
      options,
      call,
      startedAt: DateTime.now(),
      isStreaming: true,
      get isPending() {
        return !this.terminatedAt;
      },
    });
    call.operation = op;
    return call;
  }

  unary<I extends object, O extends object>(
    method: MethodInfo<I, O>,
    input: I,
    options: OperationOptions,
  ): BenchUnaryCall<I, O> {
    const call = super.unary(method, input, options) as BenchUnaryCall<I, O>;
    const op = operationsTracker.track({
      transport: this,
      method,
      request: input,
      options,
      call,
      startedAt: DateTime.now(),
      isStreaming: false,
      get isPending() {
        return !this.terminatedAt;
      },
    });
    call.operation = op;
    return call;
  }
}

//
// Authentication
//

const currentMetadata: Ref<RpcMetadata> = computed(() => {
  return {
    clientType: CLIENT_TYPE,
    clientId: clientInfo.value?.id ?? undefined,
    clientNonce: clientMeta.value?.nonce ?? undefined,
    clientAccessToken: clientInfo.value?.accessToken ?? undefined,
    badges: [],
  };
});
const currentMetadataEncoded: Ref<{ [key: string]: any }> = computed(() => {
  // flat encoding, messages as base64 :RpcMetadataEncoding
  const metadata = currentMetadata.value;
  const packed: { [key: string]: any } = {};
  if (metadata.clientType) packed["x-bench-2"] = CLIENT_TYPE.toString();
  if (metadata.clientId) packed["x-bench-3"] = metadata.clientId;
  if (metadata.clientNonce) packed["x-bench-4"] = metadata.clientNonce;
  if (metadata.clientAccessToken) packed["x-bench-5"] = metadata.clientAccessToken;
  if (metadata.badges.length > 0) {
    const packedBadges = metadata.badges.map((b) => {
      const p: { [key: string]: any } = {};
      if (b.id) p["2"] = b.id;
      if (b.key) p["3"] = b.key;
      if (b.password) p["4"] = b.password;
    });
    packed["x-bench-6"] = btoa(JSON.stringify(packedBadges));
  }
  return packed;
});

//
// Service clients
//

const _CACHED_HOST_TRANSPORTS: Ref<{ [benchId: string]: BenchGrpcWebTransport }> = shallowRef({});
const _CACHED_HOST_CLIENTS: Ref<{ [benchId: string]: HostClient }> = shallowRef({});

export const supervisorTransport = new BenchGrpcWebTransport(SUPERVISOR_URL);
export const supervisor = new SupervisorClient(supervisorTransport);

/**
 * Gets the Host for a given Bench (looking up host info via supervisor if not cached)
 * NOTE :Performance: cache resolved hosts across session in local storage?
 * NOTE :Performance: resolving hosts has a 'race condition' where the same host is resolved multiple times concurrently
 */
export async function getHostClient(bench: { id: string }): Promise<HostClient> {
  if ("id" in bench && _CACHED_HOST_CLIENTS.value[bench.id]) {
    return _CACHED_HOST_CLIENTS.value[bench.id];
  }

  const startedAt = DateTime.now();
  log.debug("host.resolve", bench);
  try {
    const { hosts: hostInfos } = await supervisor.resolveHosts({
      benches: [{ bench: { oneofKind: "id", id: bench.id } }],
    }).response;
    const hostTransport = new BenchGrpcWebTransport(`${hostInfos[0].domain}:${hostInfos[0].grpcWebPort}`);
    const hostClient = new HostClient(hostTransport);

    _CACHED_HOST_CLIENTS.value[bench.id!] = hostClient;
    _CACHED_HOST_TRANSPORTS.value[bench.id!] = hostTransport;
    triggerRef(_CACHED_HOST_CLIENTS);
    triggerRef(_CACHED_HOST_TRANSPORTS);

    log.debug("host.resolve.complete", {
      bench,
      hostInfos,
      duration: formatDuration(DateTime.now().diff(startedAt), { maxUnit: "ms" }),
    });
    return hostClient;
  } catch (e) {
    log.error("host.resolve.error", { bench, e });
    throw e;
  }
}

/** Gets a cached transport for the given scope */
export function getGraphTransport(scope?: Partial<GraphScopeData>): BenchGrpcWebTransport | undefined {
  if (scope?.benchId == null) return supervisorTransport;
  else return _CACHED_HOST_TRANSPORTS.value[scope.benchId];
}

/** Gets the Graph client for a given scope */
export async function getGraphClient(scope?: Partial<GraphScopeData>): Promise<HostClient | SupervisorClient> {
  if (scope?.benchId == null) return supervisor;
  else return await getHostClient({ id: scope.benchId });
}

/** Gets the cached Host client for a given scope */
export function getCachedHostClient(scope: GraphScopeData): HostClient {
  if (scope.benchId == null) throw new Error("no bench id");
  const hostClient = _CACHED_HOST_CLIENTS.value[scope.benchId];
  if (hostClient == null) throw new Error(`no cached host client for ${scope.benchId}`);
  return hostClient;
}

/** Gets the cached Graph client for a given scope */
export function getCachedGraphClient(scope: GraphScopeData): HostClient | SupervisorClient {
  if (scope.benchId == null) return supervisor;
  else return _CACHED_HOST_CLIENTS.value[scope.benchId];
}
