import auth from "@/language/auth";
import { BenchType, HostClient, NodeType, RpcMetadata, SupervisorClient } from "@/proto/wire";
import { SUPERVISOR_URL } from "@/utils/globals";
import { GrpcWebFetchTransport } from "@protobuf-ts/grpcweb-transport";
import { type MethodInfo, type RpcOptions, type ServerStreamingCall, type UnaryCall } from "@protobuf-ts/runtime-rpc";
import { computed, type Ref } from "vue";

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
    return super.serverStreaming(method, input, options);
  }

  unary<I extends object, O extends object>(method: MethodInfo<I, O>, input: I, options: RpcOptions): UnaryCall<I, O> {
    return super.unary(method, input, options);
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
    const packedBadges = metadata.badges.map(b => {
      const p: {[key: string]: any} = {};
      if (b.id) p['2'] = b.id;
      if (b.key) p['3'] = b.key;
      if (b.password) p['4'] = b.password;
    })
    packed["5"] = btoa(JSON.stringify(packedBadges))
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
