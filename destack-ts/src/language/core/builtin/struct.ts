import { AnyStructProto } from "@/proto";
import { BuiltinObject } from "./object";

/** A Struct is an ordered collection of Properties. */
export abstract class Struct extends BuiltinObject {

	static readonly __isStruct__: boolean = true;
	static readonly metatype: StructType;
	
}

export abstract class StructFrozen extends Struct {

	static readonly __isFrozen__: boolean = true;

	readonly _hash: number | null = null;
	readonly _repr: string | null = null;
	readonly _proto: AnyStructProto | null = null;
	readonly _value: Record<string, any> | null = null;
}
