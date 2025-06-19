import { EnumType, TypeCardinality, StructFrozen, ScalarType, StructType, QueryConnection, NodeType, EdgeType, DefaultFactory, Supergraph, Session, TraitType, Node, StringConstraint, NumberConstraint, CollectionConstraint, CascadeAction, Struct, NodeConstraint, Graph, PrimitiveType, Icon, Type, BuiltinObject, Value, NodeReference } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:50004 ==== */
export class PropertyDefinition extends StructFrozen {
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

  constructor(
    id: number,
    name: string,
    icon: Icon | null,
    description: string | null,
    cardinality: TypeCardinality,
    scalarType: ScalarType,
    primitiveType: PrimitiveType | null,
    enumType: EnumType | null,
    nodeType: NodeType | null,
    structType: StructType | null,
    keyType: Type | null,
    isRequired: boolean | null,
    isUnique: boolean | null,
    defaultValue: Value | null,
    defaultFactory: DefaultFactory | null,
    collectionConstraint: CollectionConstraint | null,
    stringConstraint: StringConstraint | null,
    numberConstraint: NumberConstraint | null,
    nodeConstraint: NodeConstraint | null,
    nodeIsCustomizable: boolean,
    edgeType: EdgeType | null,
    cascade: CascadeAction | null,
    isWired: boolean,
    isStored: boolean,
    isRepr: boolean,
    isHash: boolean,
    isEq: boolean,
    isManaged: boolean,
    isComputed: boolean,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.name = name;
    this.icon = icon;
    this.description = description;
    this.cardinality = cardinality;
    this.scalarType = scalarType;
    this.primitiveType = primitiveType;
    this.enumType = enumType;
    this.nodeType = nodeType;
    this.structType = structType;
    this.keyType = keyType;
    this.isRequired = isRequired;
    this.isUnique = isUnique;
    this.defaultValue = defaultValue;
    this.defaultFactory = defaultFactory;
    this.collectionConstraint = collectionConstraint;
    this.stringConstraint = stringConstraint;
    this.numberConstraint = numberConstraint;
    this.nodeConstraint = nodeConstraint;
    this.nodeIsCustomizable = nodeIsCustomizable;
    this.edgeType = edgeType;
    this.cascade = cascade;
    this.isWired = isWired;
    this.isStored = isStored;
    this.isRepr = isRepr;
    this.isHash = isHash;
    this.isEq = isEq;
    this.isManaged = isManaged;
    this.isComputed = isComputed;
  }


  static create(options: {
    id: number,
    name: string,
    icon?: Icon | null,
    description?: string | null,
    cardinality?: TypeCardinality,
    scalarType: ScalarType,
    primitiveType?: PrimitiveType | null,
    enumType?: EnumType | null,
    nodeType?: NodeType | null,
    structType?: StructType | null,
    keyType?: Type | null,
    isRequired?: boolean | null,
    isUnique?: boolean | null,
    defaultValue?: Value | null,
    defaultFactory?: DefaultFactory | null,
    collectionConstraint?: CollectionConstraint | null,
    stringConstraint?: StringConstraint | null,
    numberConstraint?: NumberConstraint | null,
    nodeConstraint?: NodeConstraint | null,
    nodeIsCustomizable: boolean,
    edgeType?: EdgeType | null,
    cascade?: CascadeAction | null,
    isWired: boolean,
    isStored: boolean,
    isRepr: boolean,
    isHash: boolean,
    isEq: boolean,
    isManaged: boolean,
    isComputed: boolean,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): PropertyDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new PropertyDefinition(
      options.id,
      options.name,
      options.icon ?? null,
      options.description ?? null,
      options.cardinality ?? TypeCardinality.SCALAR,
      options.scalarType,
      options.primitiveType ?? null,
      options.enumType ?? null,
      options.nodeType ?? null,
      options.structType ?? null,
      options.keyType ?? null,
      options.isRequired ?? null,
      options.isUnique ?? null,
      options.defaultValue ?? null,
      options.defaultFactory ?? null,
      options.collectionConstraint ?? null,
      options.stringConstraint ?? null,
      options.numberConstraint ?? null,
      options.nodeConstraint ?? null,
      options.nodeIsCustomizable,
      options.edgeType ?? null,
      options.cascade ?? null,
      options.isWired,
      options.isStored,
      options.isRepr,
      options.isHash,
      options.isEq,
      options.isManaged,
      options.isComputed,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50004 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50005 ==== */
export class TraitDefinition extends StructFrozen {
  readonly id: number;
  readonly type: TraitType;
  readonly name: string;
  readonly alias: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly properties: Array<PropertyDefinition>;
  readonly traits: Array<TraitType>;

  constructor(
    id: number,
    type: TraitType,
    name: string,
    alias: string,
    icon: Icon | null,
    description: string | null,
    properties: Array<PropertyDefinition>,
    traits: Array<TraitType>,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.name = name;
    this.alias = alias;
    this.icon = icon;
    this.description = description;
    this.properties = properties;
    this.traits = traits;
  }


  static create(options: {
    id: number,
    type: TraitType,
    name: string,
    alias: string,
    icon?: Icon | null,
    description?: string | null,
    properties?: Array<PropertyDefinition>,
    traits?: Array<TraitType>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): TraitDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new TraitDefinition(
      options.id,
      options.type,
      options.name,
      options.alias,
      options.icon ?? null,
      options.description ?? null,
      options.properties ?? [],
      options.traits ?? [],
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50005 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50006 ==== */
export class NodeDefinition extends StructFrozen {
  readonly id: number;
  readonly type: NodeType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly properties: Array<PropertyDefinition>;
  readonly traits: Array<TraitType>;

  constructor(
    id: number,
    type: NodeType,
    name: string,
    icon: Icon | null,
    description: string | null,
    properties: Array<PropertyDefinition>,
    traits: Array<TraitType>,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.name = name;
    this.icon = icon;
    this.description = description;
    this.properties = properties;
    this.traits = traits;
  }


  static create(options: {
    id: number,
    type: NodeType,
    name: string,
    icon?: Icon | null,
    description?: string | null,
    properties?: Array<PropertyDefinition>,
    traits?: Array<TraitType>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): NodeDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new NodeDefinition(
      options.id,
      options.type,
      options.name,
      options.icon ?? null,
      options.description ?? null,
      options.properties ?? [],
      options.traits ?? [],
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50006 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50007 ==== */
export class StructDefinition extends StructFrozen {
  readonly id: number;
  readonly type: StructType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly properties: Array<PropertyDefinition>;
  readonly isFrozen: boolean;

  constructor(
    id: number,
    type: StructType,
    name: string,
    icon: Icon | null,
    description: string | null,
    properties: Array<PropertyDefinition>,
    isFrozen: boolean,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.name = name;
    this.icon = icon;
    this.description = description;
    this.properties = properties;
    this.isFrozen = isFrozen;
  }


  static create(options: {
    id: number,
    type: StructType,
    name: string,
    icon?: Icon | null,
    description?: string | null,
    properties?: Array<PropertyDefinition>,
    isFrozen: boolean,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): StructDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new StructDefinition(
      options.id,
      options.type,
      options.name,
      options.icon ?? null,
      options.description ?? null,
      options.properties ?? [],
      options.isFrozen,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50007 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50008 ==== */
export class EnumDefinition extends StructFrozen {
  readonly id: number;
  readonly type: EnumType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;
  readonly options: Array<EnumOptionDefinition>;

  constructor(
    id: number,
    type: EnumType,
    name: string,
    icon: Icon | null,
    description: string | null,
    options: Array<EnumOptionDefinition>,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.name = name;
    this.icon = icon;
    this.description = description;
    this.options = options;
  }


  static create(options: {
    id: number,
    type: EnumType,
    name: string,
    icon?: Icon | null,
    description?: string | null,
    options?: Array<EnumOptionDefinition>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): EnumDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new EnumDefinition(
      options.id,
      options.type,
      options.name,
      options.icon ?? null,
      options.description ?? null,
      options.options ?? [],
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50008 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50009 ==== */
export class EnumOptionDefinition extends StructFrozen {
  readonly id: number;
  readonly type: EnumType;
  readonly name: string;
  readonly icon: Icon | null;
  readonly description: string | null;

  constructor(
    id: number,
    type: EnumType,
    name: string,
    icon: Icon | null,
    description: string | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.name = name;
    this.icon = icon;
    this.description = description;
  }


  static create(options: {
    id: number,
    type: EnumType,
    name: string,
    icon?: Icon | null,
    description?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): EnumOptionDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new EnumOptionDefinition(
      options.id,
      options.type,
      options.name,
      options.icon ?? null,
      options.description ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50009 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50010 ==== */
export class PermissionDefinition extends StructFrozen {
  readonly id: number;
  readonly type: EnumType;
  readonly name: string;
  readonly nodeType: NodeType;
  readonly icon: Icon | null;

  constructor(
    id: number,
    type: EnumType,
    name: string,
    nodeType: NodeType,
    icon: Icon | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.id = id;
    this.type = type;
    this.name = name;
    this.nodeType = nodeType;
    this.icon = icon;
  }


  static create(options: {
    id: number,
    type: EnumType,
    name: string,
    nodeType: NodeType,
    icon?: Icon | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): PermissionDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new PermissionDefinition(
      options.id,
      options.type,
      options.name,
      options.nodeType,
      options.icon ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50010 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50011 ==== */
export class ConstantDefinition extends StructFrozen {
  readonly name: string;
  readonly path: string;
  readonly value: Value;

  constructor(
    name: string,
    path: string,
    value: Value,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.name = name;
    this.path = path;
    this.value = value;
  }


  static create(options: {
    name: string,
    path: string,
    value: Value,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): ConstantDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new ConstantDefinition(
      options.name,
      options.path,
      options.value,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50011 ==== */