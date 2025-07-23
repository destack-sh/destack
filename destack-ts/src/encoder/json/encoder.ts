import { getObjectKey, JSON_OBJECT_ENCODERS } from "@destack/encoder/json/generate";
import { loadEncoders } from "@destack/encoder/json/generated";
import { packJson, unpackJson } from "@destack/encoder/json/wiring";
import {
  BuiltinObject,
  Encoder,
  NodeType,
  ObjectKind,
  Session,
  StructType,
  Type,
} from "@destack/language";

/** Encoder for our JSON format. */
export class JsonEncoder implements Encoder<any> {
  constructor() {
    loadEncoders();
  }

  packObject(kind: ObjectKind, metatype: NodeType | StructType, object: BuiltinObject): any {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.packObject(object);
  }

  packObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
  ): Uint8Array {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = JSON.parse(new TextDecoder().decode(value));
    return encoder.unpackObject(objectPacked, session);
  }

  packValue(value: any, type: Type): any {
    const json = packJson(value, type);
    return json;
  }

  packValueBytes(value: any, type: Type): Uint8Array {
    const json = packJson(value, type);
    return new TextEncoder().encode(JSON.stringify(json));
  }

  unpackValue(type: Type, value: any): any {
    const json = unpackJson(value, type);
    return json;
  }

  unpackValueBytes(type: Type, value: Uint8Array): any {
    const jsonValue = JSON.parse(new TextDecoder().decode(value));
    const json = unpackJson(jsonValue, type);
    return json;
  }
}
