import { HostClient, RpcMetadata, SupervisorClient } from "@/proto/wire";
import { clientInfo, clientMeta } from "@/system/local";
import { toaster } from "@/system/toast";
import { SUPERVISOR_URL } from "@/utils/globals";
import { log } from "@/utils/log";
import { GrpcWebFetchTransport } from "@protobuf-ts/grpcweb-transport";
import {
  RpcError,
  type MethodInfo,
  type RpcOptions,
  type ServerStreamingCall,
  type UnaryCall,
} from "@protobuf-ts/runtime-rpc";
import { toRef } from "@vueuse/core";
import { DateTime } from "luxon";
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
  duration?: number; // in seconds

  isStreaming: boolean;
  get isPending(): boolean;
  abort?(): void;
};

export type OperationError = RpcError | Error;

const operationsTracker = {
  RECENT_BUFFER_SIZE: 1000,

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
    const op = {
      ...opIn,
      id,
      name: `${opIn.method.service.typeName}.${opIn.method.name} [id=${id}]`,
    };
    this.pendingOps.push(op);
    this.recentOps.push(op);
    if (this.RECENT_BUFFER_SIZE > 0 && this.recentOps.length > this.RECENT_BUFFER_SIZE) {
      this.recentOps.shift();
    }
    log.debug(op.name, op.request);

    const remove = () => {
      const index = this.pendingOps.indexOf(op);
      if (index !== -1) this.pendingOps.splice(index, 1);
    };
    const onError = async (error: RpcError) => {
      const code = error.code;
      log.error(op.name, code, error);
      if (code == "UNAUTHENTICATED") {
        const { onAuthenticationError } = await import("@/system/user"); // recursive import
        onAuthenticationError(error);
      }
      toaster.error({ title: "Server error", text: error.message })
    };

    // subscribe to call events
    if ("responses" in op.call) {
      // streaming
      op.call.responses.onNext(() => {
        op.updatedAt = DateTime.now();
        op.numResponses = (op.numResponses || 0) + 1;
        log.debug(op.name, "update", op.numResponses);
      });
      op.call.responses.onComplete(() => {
        op.terminatedAt = DateTime.now();
        op.duration = op.terminatedAt.diff(op.startedAt, "seconds").seconds;
        log.debug(op.name, "completed");
        remove();
      });
      op.call.responses.onError((error) => {
        op.error = error;
        op.terminatedAt = DateTime.now();
        op.duration = op.terminatedAt.diff(op.startedAt, "seconds").seconds;
        onError(error as RpcError);
        remove();
      });
    } else {
      // unary
      op.call.response
        .then((output) => {
          op.response = output;
          log.debug(op.name, "completed", output);
        })
        .catch((error) => {
          op.error = error;
          onError(error);
        })
        .finally(() => {
          op.terminatedAt = DateTime.now();
          op.duration = op.terminatedAt.diff(op.startedAt, "seconds").seconds;
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
 * Extend the standard grpc-web fetch clients with:
 *  - authentication using our RpcMetadata
 *  - automatic retries
 *  - error logging
 *  - instrumentation
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
    options: RpcOptions,
  ): BenchServerStreamingCall<I, O> {
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
    options: RpcOptions,
  ): BenchUnaryCall<I, O> {
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

export function reactiveUnaryCall<I extends object, O extends object>(
  client: any, // TODO :Cleanup: type 'client' in reactiveUnaryCall
  method: (input: I, options?: RpcOptions) => UnaryCall<I, O>,
  input: Ref<I> | I,
  options?: RpcOptions & { noswr?: boolean },
): {
  result: Ref<O | null>;
  error: Ref<Error | null>;
  pending: Ref<Operation<I, O> | null>;
  terminated: Ref<Operation<I, O> | null>;
} {
  /** Call a unary RPC, and call again every time the input changes.*/

  const result: Ref<O | null> = shallowRef(null);
  const error: Ref<Error | null> = shallowRef(null);
  const pending: Ref<Operation<I, O> | null> = shallowRef(null);
  const terminated: Ref<Operation<I, O> | null> = shallowRef(null);

  // call method whenever input changes
  const inputRef = toRef(input);
  method = method.bind(client);
  const call = () => {
    if (options?.noswr) {
      result.value = null;
      error.value = null;
    }
    const call = method(inputRef.value, options);
    pending.value = (call as BenchUnaryCall<I, O>).operation;
    call.response
      .then((output) => {
        result.value = output;
        error.value = null;
      })
      .catch((err) => {
        result.value = null;
        error.value = err;
      })
      .finally(() => {
        terminated.value = pending.value;
        pending.value = null;
      });
  };
  watch(inputRef, call, { immediate: true });

  return { result, error, pending, terminated };
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
export async function getHostClient(bench: { id: string } | { slug: string }): Promise<HostClient> {
  /** Gets the Host for a given Bench (looking up host info via supervisor if not cached) */
  if ("slug" in bench && _CACHED_BENCH_IDS[bench.slug]) {
    return _CACHED_HOST_CLIENTS[_CACHED_BENCH_IDS[bench.slug]];
  } else if ("id" in bench && _CACHED_BENCH_IDS[bench.id]) {
    return _CACHED_HOST_CLIENTS[bench.id];
  } else {
    // fallback: lookup bench host via supervisor
    const hostInfo = await supervisor.getHost({
      bench: "id" in bench ? { id: bench.id, oneofKind: "id" } : { slug: bench.slug, oneofKind: "slug" },
    }).response;
    const hostClient = new HostClient(new BenchGrpcWebTransport({ baseUrl: hostInfo.host }));
    _CACHED_BENCH_IDS[hostInfo.benchSlug] = hostInfo.bench!.id!;
    _CACHED_HOST_CLIENTS[hostInfo.bench!.id!] = hostClient;
    return hostClient;
  }
}
