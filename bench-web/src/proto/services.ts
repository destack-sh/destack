import auth from "@/language/auth";
import { BenchType, HostClient, NodeType, RpcMetadata, SupervisorClient } from "@/proto/wire";
import { SUPERVISOR_URL } from "@/utils/globals";
import { GrpcWebFetchTransport } from "@protobuf-ts/grpcweb-transport";
import { type MethodInfo, type RpcOptions, type ServerStreamingCall, type UnaryCall } from "@protobuf-ts/runtime-rpc";
import { DateTime } from "luxon";
import { computed, type Ref } from "vue";

type Operation<I extends object, O extends object> = {
  id: number;
  name: string;
  method: MethodInfo<I, O>;
  request: I;
  response?: O; // for unary
  numResponses?: number; // for streaming
  error?: Error;
  options: RpcOptions;
  call: ServerStreamingCall<I, O> | UnaryCall<I, O>;
  startedAt: DateTime;
  updatedAt?: DateTime; // for streaming
  finishedAt?: DateTime;
  duration?: number; // in seconds
};

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
    const id = this.numTotalOps++
    const op = {
      ...opIn,
      id,
      name: `${opIn.method.service.typeName}.${opIn.method.name}:${id}`,
    }
    this.pendingOps.push(op);
    this.recentOps.push(op);
    if (this.RECENT_BUFFER_SIZE > 0 && this.recentOps.length > this.RECENT_BUFFER_SIZE) {
      this.recentOps.shift();
    }
    console.debug(op.name, op.request);

    const remove = () => {
      const index = this.pendingOps.indexOf(op);
      if (index !== -1) this.pendingOps.splice(index, 1);
    }

    // subscribe to call events
    if ("responses" in op.call) { // streaming
      op.call.responses.onNext(() => {
        op.updatedAt = DateTime.now();
        op.numResponses = (op.numResponses || 0) + 1;
      });
      op.call.responses.onComplete(() => {
        op.finishedAt = DateTime.now();
        op.duration = op.finishedAt.diff(op.startedAt, "seconds").seconds;
        console.debug(op.name, "completed");
        remove();
      });
      op.call.responses.onError((error) => {
        op.error = error;
        op.finishedAt = DateTime.now();
        op.duration = op.finishedAt.diff(op.startedAt, "seconds").seconds;
        console.error(op.name, "error", error);
        remove();
      });
    } else { // unary
      op.call.response.then((output) => {
        op.response = output;
        console.debug(op.name, "completed", output);
      }).catch((error) => {
        op.error = error;
        console.error(op.name, "error", error);
      }).finally(() => {
        op.finishedAt = DateTime.now();
        op.duration = op.finishedAt.diff(op.startedAt, "seconds").seconds;
        remove();
      });
    }
  },
};

/**
 * Extend the standard grpc-web fetch clients with:
 *  - authentication using metadata
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
  ): ServerStreamingCall<I, O> {
    const call = super.serverStreaming(method, input, options);
    operationsTracker.track({
      method,
      request: input,
      options,
      call,
      startedAt: DateTime.now(),
    });
    return call;
  }

  unary<I extends object, O extends object>(method: MethodInfo<I, O>, input: I, options: RpcOptions): UnaryCall<I, O> {
    const call = super.unary(method, input, options);
    operationsTracker.track({
      method,
      request: input,
      options,
      call,
      startedAt: DateTime.now(),
    });
    return call;
  }
}

// authentication
const currentMetadata: Ref<RpcMetadata> = computed(() => {
  return {
    clientId: auth.client.value?.id,
    clientNonce: auth.client.value?.nonce,
    badges: auth.badges,
  };
});
const currentMetadataEncoded: Ref<{ [key: string]: any }> = computed(() => {
  // flat encoding, messages as base64 :RpcMetadataEncoding
  const metadata = currentMetadata.value;
  const packed: { [key: string]: any } = {};
  if (metadata.clientId) packed["2"] = metadata.clientId;
  if (metadata.clientNonce) packed["3"] = metadata.clientNonce;
  if (metadata.clientAccessToken) packed["4"] = metadata.clientAccessToken;
  if (metadata.badges.length > 0) {
    const packedBadges = metadata.badges.map((b) => {
      const p: { [key: string]: any } = {};
      if (b.id) p["2"] = b.id;
      if (b.key) p["3"] = b.key;
      if (b.password) p["4"] = b.password;
    });
    packed["5"] = btoa(JSON.stringify(packedBadges));
  }
  return packed;
});

const TRANSPORT_FETCH_OPTIONS: Omit<RequestInit, "body" | "headers" | "method" | "signal"> = { credentials: "include" };
const supervisorTransport = new BenchGrpcWebTransport({
  baseUrl: SUPERVISOR_URL,
  fetchInit: TRANSPORT_FETCH_OPTIONS,
});
export const supervisor = new SupervisorClient(supervisorTransport);

const _CACHED_HOST_CLIENTS: { [benchId: string]: HostClient } = {};

export async function getHostClient(benchId: string): Promise<HostClient> {
  if (!_CACHED_HOST_CLIENTS[benchId]) {
    const hostInfo = await supervisor.getHost({
      bench: { metatype: BenchType.NODE_REFERENCE, type: NodeType.BENCH, id: benchId },
    });
    const hostTransport = new BenchGrpcWebTransport({
      baseUrl: hostInfo.response.host,
      fetchInit: TRANSPORT_FETCH_OPTIONS,
    });
    const host = new HostClient(hostTransport);
    _CACHED_HOST_CLIENTS[benchId] = host;
  }
  return _CACHED_HOST_CLIENTS[benchId];
}
