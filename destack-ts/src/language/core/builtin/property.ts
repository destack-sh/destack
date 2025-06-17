/** A system-defined attribute of a BuiltinObject (Struct or Node). */
export class Property {

	// meta
	readonly id: number | null;
	readonly ord: number | null;
	readonly name: string;
	readonly description: string | null;
	readonly component: BuiltinObjectBase;

	// pointers
	readonly ptrProp: Property | null;
	readonly runtimeProp: Property | null;
	readonly nodeSpaceFrom: "self" | null;
	readonly nodeIsCustomizable: boolean;
	readonly nodeHasType: boolean;
	readonly nodeHasSpace: boolean;
	readonly nodeHasDefinition: boolean;
	readonly edgeType: EdgeType | null;
	readonly cascade: CascadeAction | null;

	// flags
	readonly isUnique: boolean;
	readonly isWired: boolean;
	readonly isStored: boolean;
	readonly isRepr: boolean;
	readonly isHash: boolean;
	readonly isEq: boolean;
	readonly isManaged: boolean;
	readonly isComputed: boolean;

	// meta
	readonly ref: PropertyReference;
	readonly type: Type | null;
	readonly definition: PropertyDefinition | null;

	constructor(
		// meta
		id: number | null,
		ord: number | null,
		name: string,
		description: string | null,
		component: BuiltinObjectBase,

		// pointers
		ptrProp: Property | null,
		runtimeProp: Property | null,
		nodeSpaceFrom: "self" | null,
		nodeIsCustomizable: boolean,
		nodeHasType: boolean,
		nodeHasSpace: boolean,
		nodeHasDefinition: boolean,
		edgeType: EdgeType | null,
		cascade: CascadeAction | null,

		// flags
		isUnique: boolean,
		isWired: boolean,
		isStored: boolean,
		isRepr: boolean,
		isHash: boolean,
		isEq: boolean,
		isManaged: boolean,
		isComputed: boolean,

		// meta
		ref: PropertyReference,
		type: Type | null,
		definition: PropertyDefinition | null,
	) {
		this.id = id;
		this.ord = ord;
		this.name = name;
		this.description = description;
		this.component = component;

		this.ptrProp = ptrProp;
		this.runtimeProp = runtimeProp;
		this.nodeSpaceFrom = nodeSpaceFrom;
		this.nodeIsCustomizable = nodeIsCustomizable;
		this.nodeHasType = nodeHasType;
		this.nodeHasSpace = nodeHasSpace;
		this.nodeHasDefinition = nodeHasDefinition;
		this.edgeType = edgeType;
		this.cascade = cascade;

		this.isUnique = isUnique;
		this.isWired = isWired;
		this.isStored = isStored;
		this.isRepr = isRepr;
		this.isHash = isHash;
		this.isEq = isEq;
		this.isManaged = isManaged;
		this.isComputed = isComputed;

		this.ref = ref;
		this.type = type;
		this.definition = definition;
	}
}