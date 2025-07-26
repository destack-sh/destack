import { getObjectKey, JSON_OBJECT_ENCODERS } from "@destack/encoder/json/core";
import { loadEncoders } from "@destack/encoder/json/generated";
import { packJson, unpackJson } from "@destack/encoder/json/wiring";
import {
  type BinaryReader,
  type BinaryWriter,
  type BuiltinObject,
  type Encoder,
  NodeType,
  ObjectKind,
  type Session,
  StructType,
  type Type,
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
        `no JsonObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.packObject(object);
  }

  packObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
    writer: BinaryWriter,
  ): void {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = encoder.packObject(object);
    writer.writeJson(objectPacked);
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
        `no JsonObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.unpackObject(value, session);
  }

  unpackObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    reader: BinaryReader,
    session: Session | null,
  ): BuiltinObject {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = reader.readJson();
    return encoder.unpackObject(objectPacked, session);
  }

  packValue(value: any, type: Type): any {
    const json = packJson(type, value);
    return json;
  }

  packValueBytes(value: any, type: Type, writer: BinaryWriter): void {
    const json = packJson(type, value);
    writer.writeJson(json);
  }

  unpackValue(type: Type, value: any, session: Session | null): any {
    const json = unpackJson(type, value, session);
    return json;
  }

  unpackValueBytes(type: Type, reader: BinaryReader, session: Session | null): any {
    const jsonValue = reader.readJson();
    const json = unpackJson(type, jsonValue, session);
    return json;
  }
}
