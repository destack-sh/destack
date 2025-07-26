import type {
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

/** All the BuiltinObject encoders for our JSONC format. */
export const JSONC_OBJECT_ENCODERS: Record<string, JsoncObjectEncoder> = {};

/** A JSONC encoder for a BuiltinObject. */
export interface JsoncObjectEncoder {
  /** Pack a BuiltinObject into some encoded format. */
  packObject(object: BuiltinObject): any;

  /** Unpack a BuiltinObject from some encoded format. */
  unpackObject(value: any, session: Session | null): BuiltinObject;
}
