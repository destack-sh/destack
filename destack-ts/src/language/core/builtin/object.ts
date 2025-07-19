import type { NodeDefinition, PropertyDefinition, StructDefinition } from "@destack/language/core";
import type { Graph, QueryConnection, Session, Supergraph } from "@destack/language/core/runtime";
import type { AnyNodeProto, AnyStructProto } from "@destack/proto";

/** The base for all BuiltinObjects like Structs and Nodes and all their derivatives. */
export abstract class BuiltinObject {
  static readonly __isFrozen__: boolean;
  static readonly __isStruct__: boolean;
  static readonly __isNode__: boolean;
  static readonly __isTrait__: boolean;
  static readonly __definition__: NodeDefinition | StructDefinition;
  static readonly __properties__: Record<string, PropertyDefinition>;
  static readonly __propertiesByAlias__: Record<string, PropertyDefinition>;
  static readonly __propertiesById__: Record<number, PropertyDefinition>;

  // supergraph
  _supergraph: Supergraph | null;

  constructor(supergraph: Supergraph | null) {
    this._supergraph = supergraph;
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

  /** Clone this object. */
  clone(): BuiltinObject {
    throw new Error(`clone not implemented for ${this.constructor.name}`);
  }

  /** Get a PropertyDefinition or CustomProperty by name. */
  static property(name: string): PropertyDefinition {
    const prop = this.__propertiesByAlias__[name];
    if (prop != null) {
      return prop;
    }
    throw new Error(`property ${name} not found on ${this.constructor.name}`);
  }

  // proto

  /** Convert an instance of this BuiltinObject to a proto. */
  static __packProto__(object: BuiltinObject): AnyStructProto | AnyNodeProto {
    throw new Error(`__packProto__ not implemented for ${this.constructor.name}`);
  }

  /** Convert a proto to an instance of this BuiltinObject. */
  static __unpackProto__(
    proto: AnyStructProto | AnyNodeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null,
  ): BuiltinObject {
    throw new Error(`__unpackProto__ not implemented for ${this.constructor.name}`);
  }

  /** Convert an instance of this BuiltinObject to a proto. */
  toProto(): AnyStructProto | AnyNodeProto {
    throw new Error(`toProto not implemented for ${this.constructor.name}`);
  }

  /** Convert a proto to an instance of this BuiltinObject. */
  static fromProto(
    proto: AnyStructProto | AnyNodeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null,
  ): BuiltinObject {
    throw new Error(`fromProto not implemented for ${this.constructor.name}`);
  }

  /** Convert a binary proto string to an instance of this BuiltinObject. */
  static fromProtoString(packedProtoString: string): BuiltinObject {
    throw new Error(`fromProtoString not implemented for ${this.constructor.name}`);
  }

  // value

  /** Convert an instance of this BuiltinObject to a value. */
  static __packCson__(object: BuiltinObject): Record<string, any> {
    throw new Error(`__packCson__ not implemented for ${this.constructor.name}`);
  }

  /** Convert a value to an instance of this BuiltinObject. */
  static __unpackCson__(
    value: Record<string, any>,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null,
  ): BuiltinObject {
    throw new Error(`__unpackCson__ not implemented for ${this.constructor.name}`);
  }

  /** Convert an instance of this BuiltinObject to a value. */
  toCson(): Record<string, any> {
    throw new Error(`toCson not implemented for ${this.constructor.name}`);
  }

  /** Convert a value to an instance of this BuiltinObject. */
  static fromCson(
    value: Record<string, any>,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null,
  ): BuiltinObject {
    throw new Error(`fromCson not implemented for ${this.constructor.name}`);
  }
}

/** A BuiltinObject constructor/class. */
export type BuiltinObjectClass<
  ObjectT extends BuiltinObject = BuiltinObject,
  ProtoT extends AnyStructProto | AnyNodeProto = AnyStructProto | AnyNodeProto,
> = {
  __properties__: Record<string, PropertyDefinition>;
  __propertiesByAlias__: Record<string, PropertyDefinition>;
  __propertiesById__: Record<number, PropertyDefinition>;

  /** Convert an instance of this BuiltinObject to a proto. */
  __packProto__: (object: ObjectT) => ProtoT;

  /** Convert a proto to an instance of this BuiltinObject. */
  __unpackProto__: (
    proto: ProtoT,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null,
  ) => ObjectT;

  /** Convert an instance of this BuiltinObject to a value. */
  __packCson__: (object: ObjectT) => Record<string, any>;

  /** Convert a value to an instance of this BuiltinObject. */
  __unpackCson__: (
    value: Record<string, any>,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null,
  ) => ObjectT;
};
