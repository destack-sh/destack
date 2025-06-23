import { PropertyDefinition } from "@/language";
import { AnyNodeProto, AnyStructProto } from "@/proto";
import { Supergraph } from "../runtime/graph";

/** The base for all BuiltinObjects like Structs and Nodes and all their derivatives. */
export abstract class BuiltinObject {
  // flags
  static readonly __isFrozen__: boolean;
  static readonly __isStruct__: boolean;
  static readonly __isNode__: boolean;
  static readonly __isTrait__: boolean;

  // properties
  static readonly __properties__: Record<string, PropertyDefinition>;
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

  /** Get a property definition by name. */
  static property(name: string): PropertyDefinition {
    const prop = this.__properties__[name];
    if (!prop) {
      throw new Error(`Property ${name} not found on ${this.constructor.name}`);
    }
    return prop;
  }

  // proto

  /** Convert an instance of this BuiltinObject to a proto. */
  static __packProto__(object: BuiltinObject): AnyStructProto | AnyNodeProto {
    throw new Error(`__packProto__ not implemented for ${this.constructor.name}`);
  }

  /** Convert a proto to an instance of this BuiltinObject. */
  static __unpackProto__(proto: AnyStructProto | AnyNodeProto): BuiltinObject {
    throw new Error(`__unpackProto__ not implemented for ${this.constructor.name}`);
  }

  /** Convert an instance of this BuiltinObject to a proto. */
  toProto(): AnyStructProto | AnyNodeProto {
    throw new Error(`toProto not implemented for ${this.constructor.name}`);
  }

  /** Convert a proto to an instance of this BuiltinObject. */
  static fromProto(proto: AnyStructProto | AnyNodeProto): BuiltinObject {
    throw new Error(`fromProto not implemented for ${this.constructor.name}`);
  }

  // value

  /** Convert an instance of this BuiltinObject to a value. */
  static __packValue__(object: BuiltinObject): Record<string, any> {
    throw new Error(`__packValue__ not implemented for ${this.constructor.name}`);
  }

  /** Convert a value to an instance of this BuiltinObject. */
  static __unpackValue__(value: Record<string, any>): BuiltinObject {
    throw new Error(`__unpackValue__ not implemented for ${this.constructor.name}`);
  }

  /** Convert an instance of this BuiltinObject to a value. */
  toValue(): Record<string, any> {
    throw new Error(`toValue not implemented for ${this.constructor.name}`);
  }

  /** Convert a value to an instance of this BuiltinObject. */
  static fromValue(value: Record<string, any>): BuiltinObject {
    throw new Error(`fromValue not implemented for ${this.constructor.name}`);
  }
}
