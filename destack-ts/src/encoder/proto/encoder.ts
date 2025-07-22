import { getObjectKey, PROTO_OBJECT_ENCODERS } from "@destack/encoder/proto/generate";
import { loadEncoders } from "@destack/encoder/proto/generated";
import {
  BuiltinObject,
  Encoder,
  Graph,
  GraphConnection,
  NodeType,
  ObjectKind,
  Session,
  StructType,
  Type,
} from "@destack/language";
import { AnyNodeProto, AnyStructProto } from "@destack/proto";

/** Encoder for our protobuf format. */
export class ProtoEncoder implements Encoder<any> {
  constructor() {
    loadEncoders();
  }

  packObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): AnyStructProto | AnyNodeProto {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.packObject(options.object);
  }

  packObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): Uint8Array {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.packObjectBytes(options.object);
  }

  unpackObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    value: AnyStructProto | AnyNodeProto;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.unpackObject(options);
  }

  unpackObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    value: Uint8Array;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.unpackObjectBytes(options);
  }

  packValue(options: { value: any; type: Type }): any {
    throw new Error("not implemented");
  }

  packValueBytes(options: { value: any; type: Type }): Uint8Array {
    throw new Error("not implemented");
  }

  unpackValue(options: { type: Type; value: any }): any {
    throw new Error("not implemented");
  }

  unpackValueBytes(options: { type: Type; value: Uint8Array }): any {
    throw new Error("not implemented");
  }
}
