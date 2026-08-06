import { Handshake, type Handshake as HandshakeValue } from "../_generated/rpc/protocol/handshake.js";
import { Message, type Message as MessageValue } from "../_generated/rpc/protocol/message.js";
import { BinaryReader, BinaryWriter } from "../protocol/serde.js";
import type { Decoder, Encoder } from "./types.js";

/** Encode one complete serde value. */
export function encodeValue<Value>(codec: Encoder<Value>, value: Value): Uint8Array {
    const writer = new BinaryWriter();
    codec.encode(writer, value);

    return writer.bytes();
}

/** Decode one complete serde value. */
export function decodeValue<Value>(codec: Decoder<Value>, bytes: Uint8Array): Value {
    const reader = new BinaryReader(bytes);
    const value = codec.decode(reader);
    reader.finish();

    return value;
}

/** Encode one frozen connection handshake. */
export function encodeHandshake(value: HandshakeValue): Uint8Array {
    return encodeValue(Handshake, value);
}

/** Decode one frozen connection handshake. */
export function decodeHandshake(bytes: Uint8Array): HandshakeValue {
    return decodeValue(Handshake, bytes);
}

/** Encode one negotiated RPC message. */
export function encodeMessage(value: MessageValue): Uint8Array {
    return encodeValue(Message, value);
}

/** Decode one negotiated RPC message. */
export function decodeMessage(bytes: Uint8Array): MessageValue {
    return decodeValue(Message, bytes);
}
