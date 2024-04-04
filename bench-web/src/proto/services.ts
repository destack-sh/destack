import { GraphScope, HostClient, RpcMetadata, SupervisorClient, type IGraphIOClient } from "@/proto/wire";
import { clientInfo, clientMeta } from "@/system/client";
import { toaster } from "@/system/toast";
import { SUPERVISOR_URL } from "@/utils/globals";
import { log } from "@/utils/log";
import { formatDuration } from "@/utils/time";
import { GrpcStatusCode, GrpcWebFetchTransport } from "@protobuf-ts/grpcweb-transport";
import {
  RpcError,
  type MethodInfo,
  type RpcOptions,
  type ServerStreamingCall,
  type UnaryCall,
} from "@protobuf-ts/runtime-rpc";
import { toRef } from "@vueuse/core";
import { DateTime, Duration } from "luxon";
import { computed, shallowRef, watch, type Ref } from "vue";

/** An operation is an RPC call which may be retried. */
export type Operation<I extends object, O extends object> = {
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
const DEFAULT_RETRY_ON: GrpcStatusName[] = ["DEADLINE_EXCEEDED", "UNAVAILABLE", "INTERNAL", "UNKNOWN", "CANCELLED"];
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
  UNAVAILABLE: "Service unavailable",
  NOT_IMPLEMENTED: "Not yet supported",
  INTERNAL: "Internal server error",
  CANCELLED: "Request cancelled",
  DEADLINE_EXCEEDED: "Request timed out",
};
export const HUMANIZED_OPERATION_MESSAGE: { [key: string]: string } = {
  UNAUTHENTICATED: "Please log in and try again.",
  UNAVAILABLE: "Server could not be reached.",
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
    const serviceName = opIn.method.service.typeName.split(".").slice(2).join("."); // remove common company/project prefix
    const op = {
      ...opIn,
      id,
      name: `${serviceName}.${opIn.method.name} [id=${id}]`,
    };
    const rpcName = `rpc.${op.name}`;
    this.pendingOps.push(op);
    this.recentOps.push(op);
    if (this.recentOps.length > this.RECENT_OPERATION_BUFFER_SIZE) {
      this.recentOps.shift();
    }
    log.trace(rpcName, op.request);

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
      if (!(op.options as OperationMetadata<any>).suppressErrors) {
        log.error(rpcName, code, error, op);
        toaster.error(humanizeError(error));
      }
      if (code == "UNAUTHENTICATED") {
        const { onAuthenticationError } = await import("@/system/user"); // recursive import
        onAuthenticationError(error);
      }
    };

    // subscribe to call events
    if ("responses" in op.call) {
      // streaming
      op.call.responses.onNext(() => {
        op.updatedAt = DateTime.now();
        op.numResponses = (op.numResponses ?? 0) + 1;
        log.trace(rpcName, "update", op.numResponses);
      });
      op.call.responses.onComplete(() => {
        terminate();
        log.trace(rpcName, "completed", formatDuration(op.duration!, { maxUnit: "ms" }));
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
          log.trace(rpcName, "completed", formatDuration(op.duration!, { maxUnit: "ms" }), output);
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
 * Extend the standard grpc-web fetch clients with auth, instrumentation, retries, etc.
 */
class BenchGrpcWebTransport extends GrpcWebFetchTransport {
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
    const {
      retryOn = DEFAULT_RETRY_ON,
      maxRetries = options.retry ? undefined : 1,
      maxTimeMs,
      amendRetry,
    } = options.retry ?? {};
    
    // nocheckin: retry operations if 'retry' (on connection failure?)
    let numRetries = 0;

    const call = super.unary(method, input, options) as BenchUnaryCall<I, O>;
    const op = operationsTracker.track({
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
  if (metadata.clientId) packed["x-bench-2"] = metadata.clientId;
  if (metadata.clientNonce) packed["x-bench-3"] = metadata.clientNonce;
  if (metadata.clientAccessToken) packed["x-bench-4"] = metadata.clientAccessToken;
  if (metadata.badges.length > 0) {
    const packedBadges = metadata.badges.map((b) => {
      const p: { [key: string]: any } = {};
      if (b.id) p["2"] = b.id;
      if (b.key) p["3"] = b.key;
      if (b.password) p["4"] = b.password;
    });
    packed["x-bench-5"] = btoa(JSON.stringify(packedBadges));
  }
  return packed;
});

//
// Service clients
//

const TRANSPORT_FETCH_OPTIONS: Omit<RequestInit, "body" | "headers" | "method" | "signal"> = {};
const _CACHED_BENCH_IDS: { [slug: string]: string } = {};
const _CACHED_HOST_CLIENTS: { [benchId: string]: HostClient } = {};

export const supervisor = new SupervisorClient(
  new BenchGrpcWebTransport({
    baseUrl: SUPERVISOR_URL,
    fetchInit: TRANSPORT_FETCH_OPTIONS,
  }),
);

/** Gets the Host for a given Bench (looking up host info via supervisor if not cached) */
export async function getHostClient(bench: { id: string }): Promise<HostClient> {
  if ("id" in bench && _CACHED_BENCH_IDS[bench.id]) return _CACHED_HOST_CLIENTS[bench.id];

  // TODO :Scalability: lookup bench host via supervisor :SingleHostService
  // const hostInfo = await supervisor.getHost({
  //   bench: "id" in bench ? { id: bench.id, oneofKind: "id" } : { slug: bench.slug, oneofKind: "slug" },
  // }).response;
  const hostInfo = { host: SUPERVISOR_URL };
  const hostClient = new HostClient(new BenchGrpcWebTransport({ baseUrl: hostInfo.host }));
  _CACHED_HOST_CLIENTS[bench.id!] = hostClient;
  return hostClient;
}

/** Gets the Graph client for a given scope */
export async function getGraphClient(scope?: Partial<GraphScope>): Promise<IGraphIOClient> {
  if (scope?.benchId == null) return supervisor;
  else return await getHostClient({ id: scope.benchId });
}
