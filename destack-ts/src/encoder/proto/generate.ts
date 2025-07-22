import type {
  BuiltinObject,
  Graph,
  GraphConnection,
  NodeType,
  ObjectKind,
  Session,
  StructType,
} from "@destack/language/core";
import type { AnyNodeProto, AnyStructProto } from "@destack/proto";

/** Get a unique key for a given ObjectKind and NodeType/StructType. */
export function getObjectKey(kind: ObjectKind, metatype: NodeType | StructType) {
  return `${kind}-${metatype}`;
}

/** All the BuiltinObject encoders for our protobuf format. */
export const PROTO_OBJECT_ENCODERS: Record<string, _ProtoObjectEncoder> = {};

/** A proto encoder for a BuiltinObject. */
export interface _ProtoObjectEncoder {
  /** Pack a BuiltinObject into some encoded format. */
  packObject(object: BuiltinObject): AnyStructProto | AnyNodeProto;

  /** Unpack a BuiltinObject from some encoded format. */
  unpackObject(options: {
    value: AnyStructProto | AnyNodeProto;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject;

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  packObjectBytes(object: BuiltinObject): Uint8Array;

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  unpackObjectBytes(options: {
    value: Uint8Array;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject;
}
