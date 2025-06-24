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
} from "@destack/language/core";

/* ==== DESTACK_GENERATED_START:STRUCT:50004 ==== */
/**
 * Information about a builtin Property.
 */
export class PropertyDefinition extends StructFrozen {
  static metatype: StructType = StructType.PROPERTY_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * PropertyDefinition.id
   */
  readonly id: number;

  /**
   * PropertyDefinition.name
   */
  readonly name: string;

  /**
   * PropertyDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * PropertyDefinition.description
   */
  readonly description: string | null;

  /**
   * PropertyDefinition.cardinality
   */
  readonly cardinality: TypeCardinality;

  /**
   * PropertyDefinition.scalarType
   */
  readonly scalarType: ScalarType;

  /**
   * PropertyDefinition.primitiveType
   */
  readonly primitiveType: PrimitiveType | null;

  /**
   * PropertyDefinition.enumType
   */
  readonly enumType: EnumType | null;

  /**
   * PropertyDefinition.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * PropertyDefinition.structType
   */
  readonly structType: StructType | null;

  /**
   * PropertyDefinition.keyType
   */
  readonly keyType: Type | null;

  /**
   * PropertyDefinition.isRequired
   */
  readonly isRequired: boolean | null;

  /**
   * PropertyDefinition.isUnique
   */
  readonly isUnique: boolean | null;

  /**
   * PropertyDefinition.defaultValue
   */
  readonly defaultValue: Value | null;

  /**
   * PropertyDefinition.defaultFactory
   */
  readonly defaultFactory: DefaultFactory | null;

  /**
   * PropertyDefinition.collectionConstraint
   */
  readonly collectionConstraint: CollectionConstraint | null;

  /**
   * PropertyDefinition.stringConstraint
   */
  readonly stringConstraint: StringConstraint | null;

  /**
   * PropertyDefinition.numberConstraint
   */
  readonly numberConstraint: NumberConstraint | null;

  /**
   * PropertyDefinition.nodeConstraint
   */
  readonly nodeConstraint: NodeConstraint | null;

  /**
   * PropertyDefinition.nodeIsCustomizable
   */
  readonly nodeIsCustomizable: boolean;

  /**
   * PropertyDefinition.edgeType
   */
  readonly edgeType: EdgeType | null;

  /**
   * PropertyDefinition.cascade
   */
  readonly cascade: CascadeAction | null;

  /**
   * PropertyDefinition.isWired
   */
  readonly isWired: boolean;

  /**
   * PropertyDefinition.isStored
   */
  readonly isStored: boolean;

  /**
   * PropertyDefinition.isRepr
   */
  readonly isRepr: boolean;

  /**
   * PropertyDefinition.isHash
   */
  readonly isHash: boolean;

  /**
   * PropertyDefinition.isEq
   */
  readonly isEq: boolean;

  /**
   * PropertyDefinition.isManaged
   */
  readonly isManaged: boolean;

  /**
   * PropertyDefinition.isComputed
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`PropertyDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`PropertyDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = TypeCardinality.SCALAR;
    }
    if (_cardinality === null) {
      throw new Error(`PropertyDefinition.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`PropertyDefinition.scalarType is required`);
    }
    this.scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this.primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this.enumType = _enumType;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _isRequired = options.isRequired ?? null;
    this.isRequired = _isRequired;
    let _isUnique = options.isUnique ?? null;
    this.isUnique = _isUnique;
    let _defaultValue = options.defaultValue ?? null;
    this.defaultValue = _defaultValue;
    let _defaultFactory = options.defaultFactory ?? null;
    this.defaultFactory = _defaultFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this.nodeConstraint = _nodeConstraint;
    let _nodeIsCustomizable = options.nodeIsCustomizable;
    if (_nodeIsCustomizable === null) {
      throw new Error(`PropertyDefinition.nodeIsCustomizable is required`);
    }
    this.nodeIsCustomizable = _nodeIsCustomizable;
    let _edgeType = options.edgeType ?? null;
    this.edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this.cascade = _cascade;
    let _isWired = options.isWired;
    if (_isWired === null) {
      throw new Error(`PropertyDefinition.isWired is required`);
    }
    this.isWired = _isWired;
    let _isStored = options.isStored;
    if (_isStored === null) {
      throw new Error(`PropertyDefinition.isStored is required`);
    }
    this.isStored = _isStored;
    let _isRepr = options.isRepr;
    if (_isRepr === null) {
      throw new Error(`PropertyDefinition.isRepr is required`);
    }
    this.isRepr = _isRepr;
    let _isHash = options.isHash;
    if (_isHash === null) {
      throw new Error(`PropertyDefinition.isHash is required`);
    }
    this.isHash = _isHash;
    let _isEq = options.isEq;
    if (_isEq === null) {
      throw new Error(`PropertyDefinition.isEq is required`);
    }
    this.isEq = _isEq;
    let _isManaged = options.isManaged;
    if (_isManaged === null) {
      throw new Error(`PropertyDefinition.isManaged is required`);
    }
    this.isManaged = _isManaged;
    let _isComputed = options.isComputed;
    if (_isComputed === null) {
      throw new Error(`PropertyDefinition.isComputed is required`);
    }
    this.isComputed = _isComputed;

    // identity
    // ...
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
/**
 * Definition of a builtin Trait.
 */
export class TraitDefinition extends StructFrozen {
  static metatype: StructType = StructType.TRAIT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * TraitDefinition.id
   */
  readonly id: number;

  /**
   * TraitDefinition.type
   */
  readonly type: TraitType;

  /**
   * TraitDefinition.name
   */
  readonly name: string;

  /**
   * TraitDefinition.alias
   */
  readonly alias: string;

  /**
   * TraitDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * TraitDefinition.description
   */
  readonly description: string | null;

  /**
   * TraitDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * TraitDefinition.traits
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`TraitDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`TraitDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`TraitDefinition.name is required`);
    }
    this.name = _name;
    let _alias = options.alias;
    if (_alias === null) {
      throw new Error(`TraitDefinition.alias is required`);
    }
    this.alias = _alias;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      throw new Error(`TraitDefinition.properties is required`);
    }
    this.properties = _properties;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      throw new Error(`TraitDefinition.traits is required`);
    }
    this.traits = _traits;

    // identity
    // ...
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
/**
 * Definition of a builtin Node.
 */
export class NodeDefinition extends StructFrozen {
  static metatype: StructType = StructType.NODE_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * NodeDefinition.id
   */
  readonly id: number;

  /**
   * NodeDefinition.type
   */
  readonly type: NodeType;

  /**
   * NodeDefinition.name
   */
  readonly name: string;

  /**
   * NodeDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * NodeDefinition.description
   */
  readonly description: string | null;

  /**
   * NodeDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * NodeDefinition.traits
   */
  readonly traits: Array<TraitType>;

  /**
   * NodeDefinition.rootType
   */
  readonly rootType: NodeType | null;

  /**
   * NodeDefinition.parentTypes
   */
  readonly parentTypes: Array<NodeType>;

  /**
   * NodeDefinition.childTypes
   */
  readonly childTypes: Array<NodeType>;

  /**
   * NodeDefinition.ancestorTypes
   */
  readonly ancestorTypes: Array<NodeType>;

  /**
   * NodeDefinition.descendantTypes
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`NodeDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`NodeDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`NodeDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      throw new Error(`NodeDefinition.properties is required`);
    }
    this.properties = _properties;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      throw new Error(`NodeDefinition.traits is required`);
    }
    this.traits = _traits;
    let _rootType = options.rootType ?? null;
    this.rootType = _rootType;
    let _parentTypes = options.parentTypes ?? null;
    if (_parentTypes === null) {
      throw new Error(`NodeDefinition.parentTypes is required`);
    }
    this.parentTypes = _parentTypes;
    let _childTypes = options.childTypes ?? null;
    if (_childTypes === null) {
      throw new Error(`NodeDefinition.childTypes is required`);
    }
    this.childTypes = _childTypes;
    let _ancestorTypes = options.ancestorTypes ?? null;
    if (_ancestorTypes === null) {
      throw new Error(`NodeDefinition.ancestorTypes is required`);
    }
    this.ancestorTypes = _ancestorTypes;
    let _descendantTypes = options.descendantTypes ?? null;
    if (_descendantTypes === null) {
      throw new Error(`NodeDefinition.descendantTypes is required`);
    }
    this.descendantTypes = _descendantTypes;

    // identity
    // ...
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
/**
 * Definition of a builtin Struct.
 */
export class StructDefinition extends StructFrozen {
  static metatype: StructType = StructType.STRUCT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * StructDefinition.id
   */
  readonly id: number;

  /**
   * StructDefinition.type
   */
  readonly type: StructType;

  /**
   * StructDefinition.name
   */
  readonly name: string;

  /**
   * StructDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * StructDefinition.description
   */
  readonly description: string | null;

  /**
   * StructDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * StructDefinition.isFrozen
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`StructDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`StructDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`StructDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      throw new Error(`StructDefinition.properties is required`);
    }
    this.properties = _properties;
    let _isFrozen = options.isFrozen;
    if (_isFrozen === null) {
      throw new Error(`StructDefinition.isFrozen is required`);
    }
    this.isFrozen = _isFrozen;

    // identity
    // ...
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
/**
 * Definition of a builtin Enum.
 */
export class EnumDefinition extends StructFrozen {
  static metatype: StructType = StructType.ENUM_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * EnumDefinition.id
   */
  readonly id: number;

  /**
   * EnumDefinition.type
   */
  readonly type: EnumType;

  /**
   * EnumDefinition.name
   */
  readonly name: string;

  /**
   * EnumDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * EnumDefinition.description
   */
  readonly description: string | null;

  /**
   * EnumDefinition.options
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`EnumDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EnumDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`EnumDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _options = options.options ?? null;
    if (_options === null) {
      throw new Error(`EnumDefinition.options is required`);
    }
    this.options = _options;

    // identity
    // ...
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
/**
 * Definition of a builtin Enum Option.
 */
export class EnumOptionDefinition extends StructFrozen {
  static metatype: StructType = StructType.ENUM_OPTION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * EnumOptionDefinition.id
   */
  readonly id: number;

  /**
   * EnumOptionDefinition.type
   */
  readonly type: EnumType;

  /**
   * EnumOptionDefinition.name
   */
  readonly name: string;

  /**
   * EnumOptionDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * EnumOptionDefinition.description
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`EnumOptionDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EnumOptionDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`EnumOptionDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;

    // identity
    // ...
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
/**
 * Definition of a builtin Permission for a builtin Node.
 */
export class PermissionDefinition extends StructFrozen {
  static metatype: StructType = StructType.PERMISSION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * PermissionDefinition.id
   */
  readonly id: number;

  /**
   * PermissionDefinition.type
   */
  readonly type: EnumType;

  /**
   * PermissionDefinition.name
   */
  readonly name: string;

  /**
   * PermissionDefinition.nodeType
   */
  readonly nodeType: NodeType;

  /**
   * PermissionDefinition.icon
   */
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

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`PermissionDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`PermissionDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`PermissionDefinition.name is required`);
    }
    this.name = _name;
    let _nodeType = options.nodeType;
    if (_nodeType === null) {
      throw new Error(`PermissionDefinition.nodeType is required`);
    }
    this.nodeType = _nodeType;
    let _icon = options.icon ?? null;
    this.icon = _icon;

    // identity
    // ...
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
/**
 * Definition of a builtin Constant.
 */
export class ConstantDefinition extends StructFrozen {
  static metatype: StructType = StructType.CONSTANT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * ConstantDefinition.name
   */
  readonly name: string;

  /**
   * ConstantDefinition.path
   */
  readonly path: string;

  /**
   * ConstantDefinition.value
   */
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

    // properties
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ConstantDefinition.name is required`);
    }
    this.name = _name;
    let _path = options.path;
    if (_path === null) {
      throw new Error(`ConstantDefinition.path is required`);
    }
    this.path = _path;
    let _value = options.value;
    if (_value === null) {
      throw new Error(`ConstantDefinition.value is required`);
    }
    this.value = _value;

    // identity
    // ...
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
