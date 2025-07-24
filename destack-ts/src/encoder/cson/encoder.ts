import { CSON_OBJECT_ENCODERS, getObjectKey } from "@destack/encoder/cson/generate";
import { loadEncoders } from "@destack/encoder/cson/generated";
import { packCson, unpackCson } from "@destack/encoder/cson/wiring";
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

  packObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
    writer: BinaryWriter,
  ): void {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
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
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const objectPacked = reader.readJson();
    return encoder.unpackObject(objectPacked, session);
  }

  packValue(value: any, type: Type): any {
    const cson = packCson(value, type);
    return cson;
  }

  packValueBytes(value: any, type: Type, writer: BinaryWriter): void {
    const cson = packCson(value, type);
    writer.writeJson(cson);
  }

  unpackValue(type: Type, value: any, session: Session | null): any {
    const cson = unpackCson(value, type);
    return cson;
  }

  unpackValueBytes(type: Type, reader: BinaryReader, session: Session | null): any {
    const cson = unpackCson(reader.readJson(), type);
    return cson;
  }
}
