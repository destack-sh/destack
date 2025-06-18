import { PropertyDefinition } from "./property";
import { AnyNodeProto, AnyStructProto } from "@/proto/wire";
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

	// methods
	abstract equals(other: BuiltinObject): boolean;
	abstract hash(): number;
	abstract repr(): string;
	abstract clone(): BuiltinObject;

	/** The Supergraph this object belongs to. */
	_supergraph: Supergraph | null = null;

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
	abstract toProto(): AnyStructProto | AnyNodeProto;

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
	abstract toValue(): Record<string, any>;

	/** Convert a value to an instance of this BuiltinObject. */
	static fromValue(value: Record<string, any>): BuiltinObject {
		throw new Error(`fromValue not implemented for ${this.constructor.name}`);
	}
}