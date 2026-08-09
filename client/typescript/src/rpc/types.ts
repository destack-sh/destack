import type { Metadata } from "../_generated/rpc/call/metadata.js";
import type { BinaryReader, BinaryWriter } from "../protocol/serde.js";

/** Encoder for one statically typed RPC value. */
export interface Encoder<Value> {
    /** Encode one value. */
    encode(writer: BinaryWriter, value: Value): void;
}

/** Decoder for one statically typed RPC value. */
export interface Decoder<Value> {
    /** Decode one value. */
    decode(reader: BinaryReader): Value;
}

/** Streaming behavior of one RPC method. */
export type MethodKind =
    | "unary"
    | "serverStreaming"
    | "clientStreaming"
    | "bidirectionalStreaming";

/** Complete generated descriptor for one RPC method. */
export type Method<Request, Response, Input = never, Output = never> = {
    /** Stable service identifier. */
    readonly service: bigint;
    /** Stable method identifier. */
    readonly method: bigint;
    /** Exact method contract fingerprint. */
    readonly fingerprint: bigint;
    /** Method streaming behavior. */
    readonly kind: MethodKind;
    /** Initial request encoder. */
    readonly request: Encoder<Request>;
    /** Terminal response decoder. */
    readonly response: Decoder<Response>;
    /** Caller stream item encoder. */
    readonly input?: Encoder<Input>;
    /** Service stream item decoder. */
    readonly output?: Decoder<Output>;
};

/** Optional metadata for one RPC call. */
export type CallOptions = {
    /** Request metadata. */
    readonly metadata?: Metadata;
};

/** One initial RPC value with explicit call options. */
export class RpcRequest<Value> {
    /** Initial request value. */
    readonly value: Value;
    /** Request metadata. */
    readonly metadata: Metadata;
    /** Create one explicit RPC request. */
    constructor(value: Value, options: CallOptions = {}) {
        this.value = value;
        this.metadata = options.metadata ?? { entries: [] };
    }
}

/** Initial value accepted by a generated RPC method. */
export type RequestValue<Value> = Value | RpcRequest<Value>;

/** One successful typed RPC response. */
export type RpcResponse<Value> = {
    /** Response metadata. */
    readonly metadata: Metadata;
    /** Decoded response value. */
    readonly value: Value;
};

/** Attach explicit metadata to one initial request value. */
export function request<Value>(value: Value, options: CallOptions = {}): RpcRequest<Value> {
    return new RpcRequest(value, options);
}
