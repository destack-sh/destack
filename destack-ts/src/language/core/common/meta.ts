import {
  CascadeAction,
  CollectionConstraint,
  DefaultFactory,
  EdgeType,
  EnumType,
  Icon,
  NodeConstraint,
  NodeType,
  NumberConstraint,
  PrimitiveType,
  ScalarType,
  Session,
  StringConstraint,
  StructFrozen,
  StructType,
  Supergraph,
  TraitType,
  Type,
  TypeCardinality,
  Value,
} from "@/language";

/* ==== DESTACK_GENERATED_START:STRUCT:50004 ==== */
export class PropertyDefinition extends StructFrozen {
  static metatype: StructType = StructType.PROPERTY_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly cardinality: TypeCardinality;
  readonly scalarType: ScalarType;
  readonly primitiveType: PrimitiveType | null;
  readonly enumType: EnumType | null;
  readonly nodeType: NodeType | null;
  readonly structType: StructType | null;
  readonly keyType: Type | null;
  readonly isRequired: boolean | null;
  readonly isUnique: boolean | null;
  readonly defaultValue: Value | null;
  readonly defaultFactory: DefaultFactory | null;
  readonly collectionConstraint: CollectionConstraint | null;
  readonly stringConstraint: StringConstraint | null;
  readonly numberConstraint: NumberConstraint | null;
  readonly nodeConstraint: NodeConstraint | null;
  readonly nodeIsCustomizable: boolean;
  readonly edgeType: EdgeType | null;
  readonly cascade: CascadeAction | null;
  readonly isWired: boolean;
  readonly isStored: boolean;
  readonly isRepr: boolean;
  readonly isHash: boolean;
  readonly isEq: boolean;
  readonly isManaged: boolean;
  readonly isComputed: boolean;

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    structType?: StructType | null;
    keyType?: Type | null;
    isRequired?: boolean | null;
    isUnique?: boolean | null;
    defaultValue?: Value | null;
    defaultFactory?: DefaultFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
    nodeIsCustomizable: boolean;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isWired: boolean;
    isStored: boolean;
    isRepr: boolean;
    isHash: boolean;
    isEq: boolean;
    isManaged: boolean;
    isComputed: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.description = options.description ?? null;
    this.cardinality = options.cardinality ?? TypeCardinality.SCALAR;
    this.scalarType = options.scalarType;
    this.primitiveType = options.primitiveType ?? null;
    this.enumType = options.enumType ?? null;
    this.nodeType = options.nodeType ?? null;
    this.structType = options.structType ?? null;
    this.keyType = options.keyType ?? null;
    this.isRequired = options.isRequired ?? null;
    this.isUnique = options.isUnique ?? null;
    this.defaultValue = options.defaultValue ?? null;
    this.defaultFactory = options.defaultFactory ?? null;
    this.collectionConstraint = options.collectionConstraint ?? null;
    this.stringConstraint = options.stringConstraint ?? null;
    this.numberConstraint = options.numberConstraint ?? null;
    this.nodeConstraint = options.nodeConstraint ?? null;
    this.nodeIsCustomizable = options.nodeIsCustomizable;
    this.edgeType = options.edgeType ?? null;
    this.cascade = options.cascade ?? null;
    this.isWired = options.isWired;
    this.isStored = options.isStored;
    this.isRepr = options.isRepr;
    this.isHash = options.isHash;
    this.isEq = options.isEq;
    this.isManaged = options.isManaged;
    this.isComputed = options.isComputed;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50004 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50005 ==== */
export class TraitDefinition extends StructFrozen {
  static metatype: StructType = StructType.TRAIT_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly type: TraitType;
  readonly name: string;
  readonly alias: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly properties: Array<PropertyDefinition>;
  readonly traits: Array<TraitType>;

  constructor(options: {
    id: number;
    type: TraitType;
    name: string;
    alias: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    traits?: Array<TraitType>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.alias = options.alias;
    this.icon = options.icon ?? null;
    this.description = options.description ?? null;
    this.properties = options.properties ?? [];
    this.traits = options.traits ?? [];
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50005 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50006 ==== */
export class NodeDefinition extends StructFrozen {
  static metatype: StructType = StructType.NODE_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly type: NodeType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly properties: Array<PropertyDefinition>;
  readonly traits: Array<TraitType>;
  readonly rootType: NodeType | null;
  readonly parentTypes: Array<NodeType>;
  readonly childTypes: Array<NodeType>;
  readonly ancestorTypes: Array<NodeType>;
  readonly descendantTypes: Array<NodeType>;

  constructor(options: {
    id: number;
    type: NodeType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    traits?: Array<TraitType>;
    rootType?: NodeType | null;
    parentTypes?: Array<NodeType>;
    childTypes?: Array<NodeType>;
    ancestorTypes?: Array<NodeType>;
    descendantTypes?: Array<NodeType>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.description = options.description ?? null;
    this.properties = options.properties ?? [];
    this.traits = options.traits ?? [];
    this.rootType = options.rootType ?? null;
    this.parentTypes = options.parentTypes ?? [];
    this.childTypes = options.childTypes ?? [];
    this.ancestorTypes = options.ancestorTypes ?? [];
    this.descendantTypes = options.descendantTypes ?? [];
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50006 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50007 ==== */
export class StructDefinition extends StructFrozen {
  static metatype: StructType = StructType.STRUCT_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly type: StructType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly properties: Array<PropertyDefinition>;
  readonly isFrozen: boolean;

  constructor(options: {
    id: number;
    type: StructType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    isFrozen: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.description = options.description ?? null;
    this.properties = options.properties ?? [];
    this.isFrozen = options.isFrozen;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50007 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50008 ==== */
export class EnumDefinition extends StructFrozen {
  static metatype: StructType = StructType.ENUM_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly type: EnumType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly options: Array<EnumOptionDefinition>;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    options?: Array<EnumOptionDefinition>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.description = options.description ?? null;
    this.options = options.options ?? [];
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50008 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50009 ==== */
export class EnumOptionDefinition extends StructFrozen {
  static metatype: StructType = StructType.ENUM_OPTION_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly type: EnumType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.description = options.description ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50009 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50010 ==== */
export class PermissionDefinition extends StructFrozen {
  static metatype: StructType = StructType.PERMISSION_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly id: number;
  readonly type: EnumType;
  readonly name: string;
  readonly nodeType: NodeType;
  readonly icon: Icon | null;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    nodeType: NodeType;
    icon?: Icon | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.nodeType = options.nodeType;
    this.icon = options.icon ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50010 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50011 ==== */
export class ConstantDefinition extends StructFrozen {
  static metatype: StructType = StructType.CONSTANT_DEFINITION;
  static __isFrozen__: boolean = true;

  readonly name: string;
  readonly path: string;
  readonly value: Value;

  constructor(options: {
    name: string;
    path: string;
    value: Value;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.name = options.name;
    this.path = options.path;
    this.value = options.value;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50011 ==== */
