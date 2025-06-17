/** The base for all BuiltinObjects like Structs and Nodes and all their derivatives. */
abstract class BuiltinObjectBase {

	static isFrozen: boolean;
	static isStruct: boolean;
	static isNode: boolean;
	static isTrait: boolean;

	static properties: Record<string, Property>;
	static propertiesById: Record<number, Property>;
}