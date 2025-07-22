import type {
  BuiltinObject,
  Graph,
  GraphConnection,
  NodeType,
  ObjectKind,
  Session,
  StructType,
  Type,
} from "@destack/language";

export interface Encoder<T> {
  /** Pack a BuiltinObject into some encoded format. */
  packObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): T;

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  packObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): Uint8Array;

  /** Unpack a BuiltinObject from some encoded format. */
  unpackObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    value: T;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject;

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  unpackObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    value: Uint8Array;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject;

  /** Pack a value into some encoded format. */
  packValue(options: { value: any; type: Type }): T;

  /** Pack a value into the byte representation of its encoded format. */
  packValueBytes(options: { value: any; type: Type }): Uint8Array;

  /** Unpack a value from some encoded format. */
  unpackValue(options: {
    type: Type;
    value: T;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): any;

  /** Unpack a value from the byte representation of its encoded format. */
  unpackValueBytes(options: {
    type: Type;
    value: Uint8Array;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): any;
}
