import {
  ENCODERS,
  Encoding,
  type NodeDefinition,
  NodeType,
  ObjectKind,
  type PropertyDefinition,
  type StructDefinition,
  StructType,
} from "@destack/language/core";
import type { Session } from "@destack/language/core/runtime";

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

  constructor(_session: Session | null) {
    this._session = _session;
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
    return encoder.packObject(
      (this.constructor as typeof BuiltinObject).__kind__,
      this.metatype,
      this,
    );
  }

  /** Pack a BuiltinObject into some encoded format. */
  static pack(encoding: Encoding, object: BuiltinObject): any {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObject(this.__kind__, this.metatype, object);
  }

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  packBytes(encoding: Encoding): Uint8Array {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObjectBytes(
      (this.constructor as typeof BuiltinObject).__kind__,
      this.metatype,
      this,
    );
  }

  /** Pack a BuiltinObject into the byte representation of its encoded format. */
  static packBytes(encoding: Encoding, object: BuiltinObject): Uint8Array {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.packObjectBytes(this.__kind__, this.metatype, object);
  }

  /** Unpack a BuiltinObject from some encoded format. */
  static unpack(encoding: Encoding, value: any, session?: Session | null): BuiltinObject {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.unpackObject(this.__kind__, this.metatype, value, session ?? null);
  }

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  static unpackBytes(
    encoding: Encoding,
    value: Uint8Array,
    session?: Session | null,
  ): BuiltinObject {
    const encoder = ENCODERS[encoding];
    if (encoder == null) {
      throw new Error(`no Encoder defined for ${Encoding[encoding]!}`);
    }
    return encoder.unpackObjectBytes(this.__kind__, this.metatype, value, session ?? null);
  }

  /** Unpack a BuiltinObject from the base64-encoded byte representation of its encoded format. */
  static unpackBytesBase64(
    encoding: Encoding,
    value: string,
    session?: Session | null,
  ): BuiltinObject {
    const valueBytes = Buffer.from(value, "base64");
    return this.unpackBytes(encoding, valueBytes, session ?? null);
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
  unpack(encoding: Encoding, value: any, session?: Session | null): BuiltinObject;

  /** Unpack a BuiltinObject from the byte representation of its encoded format. */
  unpackBytes(encoding: Encoding, value: Uint8Array, session?: Session | null): BuiltinObject;

  /** Unpack a BuiltinObject from the base64-encoded byte representation of its encoded format. */
  unpackBytesBase64(encoding: Encoding, value: string, session?: Session | null): BuiltinObject;
};

/** A cached packed representation of a BuiltinObject. */
export type PackedCache = {
  encoding: Encoding;
  isBytes: boolean;
  packed: any;
};
