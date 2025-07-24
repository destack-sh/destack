import type {
  BinaryReader,
  BinaryWriter,
  BuiltinObject,
  NodeType,
  ObjectKind,
  Session,
  StructType,
} from "@destack/language/core";

/** Get a unique key for a given ObjectKind and NodeType/StructType. */
export function getObjectKey(kind: ObjectKind, metatype: NodeType | StructType) {
  return `${kind}-${metatype}`;
}

/** All the BuiltinObject encoders for our Kompakt format. */
export const KOMPAKT_OBJECT_ENCODERS: Record<string, KompaktObjectEncoder> = {};

/** A Kompakt encoder for a BuiltinObject. */
export interface KompaktObjectEncoder {
  /** Pack a BuiltinObject into some encoded format. */
  packObject(object: BuiltinObject, writer: BinaryWriter): void;

  /** Unpack a BuiltinObject from some encoded format. */
  unpackObject(reader: BinaryReader, session: Session | null): BuiltinObject;
}
