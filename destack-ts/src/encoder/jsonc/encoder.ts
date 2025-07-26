import { getObjectKey, JSONC_OBJECT_ENCODERS } from "@destack/encoder/jsonc/core";
import { loadEncoders } from "@destack/encoder/jsonc/generated";
import { packJsonc, unpackJsonc } from "@destack/encoder/jsonc/wiring";
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

/** Encoder for our JSONC format. */
export class JsoncEncoder implements Encoder<any> {
  constructor() {
    loadEncoders();
  }

  packObject(kind: ObjectKind, metatype: NodeType | StructType, object: BuiltinObject): any {
    const encoder = JSONC_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsoncObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = JSONC_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsoncObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = JSONC_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsoncObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = JSONC_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no JsoncObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = reader.readJson();
    return encoder.unpackObject(objectPacked, session);
  }

  packValue(value: any, type: Type): any {
    const jsonc = packJsonc(value, type);
    return jsonc;
  }

  packValueBytes(value: any, type: Type, writer: BinaryWriter): void {
    const jsonc = packJsonc(value, type);
    writer.writeJson(jsonc);
  }

  unpackValue(type: Type, value: any, session: Session | null): any {
    const jsonc = unpackJsonc(value, type);
    return jsonc;
  }

  unpackValueBytes(type: Type, reader: BinaryReader, session: Session | null): any {
    const jsonc = unpackJsonc(reader.readJson(), type);
    return jsonc;
  }
}
