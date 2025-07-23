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
  BinaryWriter,
  BinaryReader,
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
    writer: BinaryWriter,
  ): void {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
        `no JsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = reader.readJson();
    return encoder.unpackObject(objectPacked, session);
  }

  packValue(value: any, type: Type): any {
    const json = packJson(value, type);
    return json;
  }

  packValueBytes(value: any, type: Type, writer: BinaryWriter): void {
    const json = packJson(value, type);
    writer.writeJson(json);
  }

  unpackValue(type: Type, value: any, session: Session | null): any {
    const json = unpackJson(value, type, session);
    return json;
  }

  unpackValueBytes(type: Type, reader: BinaryReader, session: Session | null): any {
    const jsonValue = reader.readJson();
    const json = unpackJson(jsonValue, type, session);
    return json;
  }
}
