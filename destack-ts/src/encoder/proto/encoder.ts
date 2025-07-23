import { getObjectKey, PROTO_OBJECT_ENCODERS } from "@destack/encoder/proto/generate";
import { loadEncoders } from "@destack/encoder/proto/generated";
import {
  BuiltinObject,
  Encoder,
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

  packObject(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
  ): AnyStructProto | AnyNodeProto {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.packObject(object);
  }

  packObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
  ): Uint8Array {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.packObjectBinary(object);
  }

  unpackObject(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: AnyStructProto | AnyNodeProto,
    session: Session | null,
  ): BuiltinObject {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.unpackObject(value, session);
  }

  unpackObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: Uint8Array,
    session: Session | null,
  ): BuiltinObject {
    const encoder = PROTO_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no ProtoEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.unpackObjectBinary(value, session);
  }

  packValue(value: any, type: Type): any {
    throw new Error("not implemented");
  }

  packValueBytes(value: any, type: Type): Uint8Array {
    throw new Error("not implemented");
  }

  unpackValue(type: Type, value: any): any {
    throw new Error("not implemented");
  }

  unpackValueBytes(type: Type, value: Uint8Array): any {
    throw new Error("not implemented");
  }
}
