import { getObjectKey, KOMPAKT_OBJECT_ENCODERS } from "@destack/encoder/kompakt/core";
import {
  BinaryReader,
  BinaryWriter,
  type BuiltinObject,
  type Encoder,
  NodeType,
  ObjectKind,
  type Session,
  StructType,
  type Type,
} from "@destack/language/core";

/** Encoder for our Kompakt format. */
export class KompaktEncoder implements Encoder<Uint8Array> {
  packObject(kind: ObjectKind, metatype: NodeType | StructType, object: BuiltinObject): Uint8Array {
    const encoder = KOMPAKT_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no KompaktObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const writer = new BinaryWriter();
    encoder.packObject(object, writer);
    return writer.toBytes();
  }

  packObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
    writer: BinaryWriter,
  ): void {
    const encoder = KOMPAKT_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no KompaktObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    encoder.packObject(object, writer);
  }

  unpackObject(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: Uint8Array,
    session: Session | null,
  ): BuiltinObject {
    const encoder = KOMPAKT_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no KompaktObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    const reader = new BinaryReader(value);
    return encoder.unpackObject(reader, session);
  }

  unpackObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    reader: BinaryReader,
    session: Session | null,
  ): BuiltinObject {
    const encoder = KOMPAKT_OBJECT_ENCODERS[getObjectKey(kind, metatype)];
    if (!encoder) {
      throw new Error(
        `no KompaktObjectEncoder for ${ObjectKind[kind] ?? kind}:${NodeType[metatype] ?? StructType[metatype] ?? metatype}`,
      );
    }
    return encoder.unpackObject(reader, session);
  }

  packValue(value: any, type: Type): Uint8Array {
    throw new Error("not implemented");
  }

  packValueBytes(value: any, type: Type, writer: BinaryWriter): void {
    throw new Error("not implemented");
  }

  unpackValue(type: Type, value: Uint8Array, session: Session | null): any {
    throw new Error("not implemented");
  }

  unpackValueBytes(type: Type, reader: BinaryReader, session: Session | null): any {
    throw new Error("not implemented");
  }
}
