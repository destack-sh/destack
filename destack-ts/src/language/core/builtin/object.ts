import {
  ENCODERS,
  Encoding,
  NodeType,
  ObjectKind,
  StructType,
  type NodeDefinition,
  type PropertyDefinition,
  type StructDefinition,
} from "@destack/language/core";
import type { Graph, GraphConnection, Session } from "@destack/language/core/runtime";

/** The base for all BuiltinObjects like Structs and Nodes and all their derivatives. */
export abstract class BuiltinObject {
  static readonly metatype: NodeType | StructType;
  static readonly __kind__: ObjectKind;

  static readonly __isFrozen__: boolean;
  static readonly __isStruct__: boolean;
  static readonly __isNode__: boolean;
  static readonly __isTrait__: boolean;
  static readonly __definition__: NodeDefinition | StructDefinition;
  static readonly __properties__: Record<string, PropertyDefinition>;
  static readonly __propertiesByAlias__: Record<string, PropertyDefinition>;
  static readonly __propertiesById__: Record<number, PropertyDefinition>;

  /* The Session this BuiltinObject is in. */
  _session: Session | null;
  /* The Graph this BuiltinObject is in. */
  _graph: Graph | null;

  constructor(_session: Session | null, _graph: Graph | null) {
    this._session = _session;
    this._graph = _graph;
  }

  get metatype(): NodeType | StructType {
    return (this.constructor as typeof BuiltinObject).metatype;
  }

  // methods

  /** Check if this object is equal to another object. */
  equals(other: BuiltinObject): boolean {
    throw new Error(`equals not implemented for ${this.constructor.name}`);
  }

  /** Get a hash of this object. */
  hash(): number {
    throw new Error(`hash not implemented for ${this.constructor.name}`);
  }

  /** Get a string representation of this object. */
  repr(): string {
    throw new Error(`repr not implemented for ${this.constructor.name}`);
  }

  /** Get a PropertyDefinition or CustomProperty by name. */
  static property(name: string): PropertyDefinition {
    const prop = this.__propertiesByAlias__[name];
    if (prop != null) {
      return prop;
    }
    throw new Error(`property ${name} not found on ${this.constructor.name}`);
  }

  // encoding

  /** Pack this BuiltinObject into some encoded format. */
  pack(encoding: Encoding): any {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObject({
      kind: (this.constructor as typeof BuiltinObject).__kind__,
      metatype: (this.constructor as typeof BuiltinObject).metatype,
      object: this,
    });
  }

  /** Pack a BuiltinObject into some encoded format. */
  static pack(encoding: Encoding, object: BuiltinObject): any {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObject({
      kind: (this.constructor as typeof BuiltinObject).__kind__,
      metatype: (this.constructor as typeof BuiltinObject).metatype,
      object,
    });
  }

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  packBytes(encoding: Encoding): Uint8Array {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObjectBytes({
      kind: (this.constructor as typeof BuiltinObject).__kind__,
      metatype: (this.constructor as typeof BuiltinObject).metatype,
      object: this,
    });
  }

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  static packBytes(encoding: Encoding, object: BuiltinObject): Uint8Array {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObjectBytes({
      kind: this.__kind__,
      metatype: this.metatype,
      object,
    });
  }

  /** Unpack a BuiltinObject from some encoded format. */
  static unpack(options: {
    encoding: Encoding;
    value: any;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }): BuiltinObject {
    const encoder = ENCODERS[options.encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[options.encoding]!}`);
    }
    return encoder.unpackObject({
      kind: this.__kind__,
      metatype: this.metatype,
      value: options.value,
      _session: options._session ?? null,
      _graph: options._graph ?? null,
      _connection: options._connection ?? null,
    });
  }

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  static unpackBytes(options: {
    encoding: Encoding;
    value: Uint8Array;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }): BuiltinObject {
    const encoder = ENCODERS[options.encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[options.encoding]!}`);
    }
    return encoder.unpackObjectBytes({
      kind: this.__kind__,
      metatype: this.metatype,
      value: options.value,
      _session: options._session ?? null,
      _graph: options._graph ?? null,
      _connection: options._connection ?? null,
    });
  }

  /** Unpack a BuiltinObject from the base64-encoded byte representation of its encoded format. */
  static unpackBytesBase64(options: {
    encoding: Encoding;
    value: string;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }): BuiltinObject {
    const valueBytes = Buffer.from(options.value, "base64");
    return this.unpackBytes({
      encoding: options.encoding,
      value: valueBytes,
      _session: options._session,
      _graph: options._graph,
      _connection: options._connection,
    });
  }
}

/** A BuiltinObject constructor/class. */
export type BuiltinObjectClass<ObjectT extends BuiltinObject = BuiltinObject> = {
  __properties__: Record<string, PropertyDefinition>;
  __propertiesByAlias__: Record<string, PropertyDefinition>;
  __propertiesById__: Record<number, PropertyDefinition>;

  /** Pack this BuiltinObject into some encoded format. */
  pack(encoding: Encoding, object: ObjectT): any;

  /** Pack this BuiltinObject into the byte representation of its encoded format. */
  packBytes(encoding: Encoding, object: ObjectT): Uint8Array;

  /** Unpack a BuiltinObject from some encoded format. */
  unpack(options: {
    encoding: Encoding;
    value: any;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }): BuiltinObject;

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  unpackBytes(options: {
    encoding: Encoding;
    value: Uint8Array;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }): BuiltinObject;

  /** Unpack a BuiltinObject from the base64-encoded byte representation of its encoded format. */
  unpackBytesBase64(options: {
    encoding: Encoding;
    value: string;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }): BuiltinObject;
};

/** A cached packed representation of a BuiltinObject. */
export type PackedCache = {
  encoding: Encoding;
  isBytes: boolean;
  packed: any;
};
