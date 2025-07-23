import { CSON_OBJECT_ENCODERS, getObjectKey } from "@destack/encoder/cson/generate";
import { loadEncoders } from "@destack/encoder/cson/generated";
import { packCson, unpackCson } from "@destack/encoder/cson/wiring";
import {
  BuiltinObject,
  Encoder,
  NodeType,
  ObjectKind,
  Session,
  StructType,
  Type,
} from "@destack/language";

/** Encoder for our CSON format. */
export class CsonEncoder implements Encoder<any> {
  constructor() {
    loadEncoders();
  }

  packObject(kind: ObjectKind, metatype: NodeType | StructType, object: BuiltinObject): any {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.packObject(object);
  }

  packObjectBytes(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
  ): Uint8Array {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = encoder.packObject(object);
    return new TextEncoder().encode(JSON.stringify(objectPacked));
  }

  unpackObject(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: any,
    session: Session | null,
  ): BuiltinObject {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.unpackObject(value, session);
  }

  unpackObjectBytes(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: Uint8Array,
    session: Session | null,
  ): BuiltinObject {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = JSON.parse(new TextDecoder().decode(value));
    return encoder.unpackObject(objectPacked, session);
  }

  packValue(value: any, type: Type): any {
    const cson = packCson(value, type);
    return cson;
  }

  packValueBytes(value: any, type: Type): Uint8Array {
    const cson = packCson(value, type);
    return new TextEncoder().encode(JSON.stringify(cson));
  }

  unpackValue(type: Type, value: any, session: Session | null): any {
    const cson = unpackCson(value, type);
    return cson;
  }

  unpackValueBytes(type: Type, value: Uint8Array, session: Session | null): any {
    const cson = unpackCson(value, type);
    return cson;
  }
}
