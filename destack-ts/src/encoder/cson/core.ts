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

/** All the BuiltinObject encoders for our CSON format. */
export const CSON_OBJECT_ENCODERS: Record<string, CsonObjectEncoder> = {};

/** A CSON encoder for a BuiltinObject. */
export interface CsonObjectEncoder {
  /** Pack a BuiltinObject into some encoded format. */
  packObject(object: BuiltinObject): any;

  /** Unpack a BuiltinObject from some encoded format. */
  unpackObject(value: any, session: Session | null): BuiltinObject;
}
