import type {
  BuiltinObject,
  NodeType,
  ObjectKind,
  Session,
  StructType,
  Type,
} from "@destack/language";

export interface Encoder<T> {
  /** Pack a BuiltinObject into some encoded format. */
  packObject(kind: ObjectKind, metatype: NodeType | StructType, object: BuiltinObject): T;

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  packObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    object: BuiltinObject,
  ): Uint8Array;

  /** Unpack a BuiltinObject from some encoded format. */
  unpackObject(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: T,
    session: Session | null,
  ): BuiltinObject;

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  unpackObjectBinary(
    kind: ObjectKind,
    metatype: NodeType | StructType,
    value: Uint8Array,
    session: Session | null,
  ): BuiltinObject;

  /** Pack a value into some encoded format. */
  packValue(value: any, type: Type): T;

  /** Pack a value into the byte representation of its encoded format. */
  packValueBytes(value: any, type: Type): Uint8Array;

  /** Unpack a value from some encoded format. */
  unpackValue(type: Type, value: T, session: Session | null): any;

  /** Unpack a value from the byte representation of its encoded format. */
  unpackValueBytes(type: Type, value: Uint8Array, session: Session | null): any;
}
