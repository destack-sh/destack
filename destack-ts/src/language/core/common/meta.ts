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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = PropertyDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PropertyDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50004;
    objectValue["2"] = object.id;
    objectValue["31"] = object.name;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description !== null) {
      objectValue["36"] = object.description;
    }
    objectValue["40"] = object.cardinality;
    objectValue["41"] = object.scalarType;
    if (object.primitiveType !== null) {
      objectValue["42"] = object.primitiveType;
    }
    if (object.enumType !== null) {
      objectValue["43"] = object.enumType;
    }
    if (object.nodeType !== null) {
      objectValue["44"] = object.nodeType;
    }
    if (object.structType !== null) {
      objectValue["46"] = object.structType;
    }
    if (object.keyType !== null) {
      objectValue["48"] = object.keyType.toValue();
    }
    if (object.isRequired !== null) {
      objectValue["50"] = object.isRequired;
    }
    if (object.isUnique !== null) {
      objectValue["51"] = object.isUnique;
    }
    if (object.defaultValue !== null) {
      objectValue["55"] = object.defaultValue.toValue();
    }
    if (object.defaultFactory !== null) {
      objectValue["56"] = object.defaultFactory;
    }
    if (object.collectionConstraint !== null) {
      objectValue["60"] = object.collectionConstraint.toValue();
    }
    if (object.stringConstraint !== null) {
      objectValue["61"] = object.stringConstraint.toValue();
    }
    if (object.numberConstraint !== null) {
      objectValue["62"] = object.numberConstraint.toValue();
    }
    if (object.nodeConstraint !== null) {
      objectValue["63"] = object.nodeConstraint.toValue();
    }
    objectValue["73"] = object.nodeIsCustomizable;
    if (object.edgeType !== null) {
      objectValue["74"] = object.edgeType;
    }
    if (object.cascade !== null) {
      objectValue["75"] = object.cascade;
    }
    objectValue["80"] = object.isWired;
    objectValue["81"] = object.isStored;
    objectValue["82"] = object.isRepr;
    objectValue["83"] = object.isHash;
    objectValue["84"] = object.isEq;
    objectValue["85"] = object.isManaged;
    objectValue["86"] = object.isComputed;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue !== undefined ? descriptionValue : null;
    const primitiveTypeValue = objectValue["42"];
    const unpackedPrimitiveType = primitiveTypeValue !== undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["43"];
    const unpackedEnumType = enumTypeValue !== undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["44"];
    const unpackedNodeType = nodeTypeValue !== undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["46"];
    const unpackedStructType = structTypeValue !== undefined ? Number(structTypeValue) : null;
    const keyTypeValue = objectValue["48"];
    const unpackedKeyType =
      keyTypeValue !== undefined ? Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection) : null;
    const isRequiredValue = objectValue["50"];
    const unpackedIsRequired = isRequiredValue !== undefined ? isRequiredValue : null;
    const isUniqueValue = objectValue["51"];
    const unpackedIsUnique = isUniqueValue !== undefined ? isUniqueValue : null;
    const defaultValueValue = objectValue["55"];
    const unpackedDefaultValue =
      defaultValueValue !== undefined
        ? Value.fromValue(defaultValueValue, _session, _supergraph, _graph, _connection)
        : null;
    const defaultFactoryValue = objectValue["56"];
    const unpackedDefaultFactory = defaultFactoryValue !== undefined ? Number(defaultFactoryValue) : null;
    const collectionConstraintValue = objectValue["60"];
    const unpackedCollectionConstraint =
      collectionConstraintValue !== undefined
        ? CollectionConstraint.fromValue(collectionConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const stringConstraintValue = objectValue["61"];
    const unpackedStringConstraint =
      stringConstraintValue !== undefined
        ? StringConstraint.fromValue(stringConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const numberConstraintValue = objectValue["62"];
    const unpackedNumberConstraint =
      numberConstraintValue !== undefined
        ? NumberConstraint.fromValue(numberConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodeConstraintValue = objectValue["63"];
    const unpackedNodeConstraint =
      nodeConstraintValue !== undefined
        ? NodeConstraint.fromValue(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const edgeTypeValue = objectValue["74"];
    const unpackedEdgeType = edgeTypeValue !== undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectValue["75"];
    const unpackedCascade = cascadeValue !== undefined ? Number(cascadeValue) : null;
    return new PropertyDefinition({
      id: Number(objectValue["2"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      cardinality: Number(objectValue["40"]),
      scalarType: Number(objectValue["41"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      structType: unpackedStructType,
      keyType: unpackedKeyType,
      isRequired: unpackedIsRequired,
      isUnique: unpackedIsUnique,
      defaultValue: unpackedDefaultValue,
      defaultFactory: unpackedDefaultFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
      nodeIsCustomizable: objectValue["73"],
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      isWired: objectValue["80"],
      isStored: objectValue["81"],
      isRepr: objectValue["82"],
      isHash: objectValue["83"],
      isEq: objectValue["84"],
      isManaged: objectValue["85"],
      isComputed: objectValue["86"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    return PropertyDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = TraitDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: TraitDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50005;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["32"] = object.alias;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description !== null) {
      objectValue["36"] = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["50"] = packedProperties;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectValue["51"] = packedTraits;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue !== undefined ? descriptionValue : null;
    const unpackedProperties: any[] = [];
    if (objectValue["50"] !== undefined) {
      for (const item of objectValue["50"]) {
        unpackedProperties.push(PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedTraits: any[] = [];
    if (objectValue["51"] !== undefined) {
      for (const item of objectValue["51"]) {
        unpackedTraits.push(Number(item));
      }
    }
    return new TraitDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      alias: objectValue["32"],
      icon: unpackedIcon,
      description: unpackedDescription,
      properties: unpackedProperties,
      traits: unpackedTraits,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    return TraitDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = NodeDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NodeDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50006;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description !== null) {
      objectValue["36"] = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["50"] = packedProperties;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectValue["51"] = packedTraits;
    }
    if (object.rootType !== null) {
      objectValue["52"] = object.rootType;
    }
    if (object.parentTypes) {
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(item);
      }
      objectValue["53"] = packedParentTypes;
    }
    if (object.childTypes) {
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(item);
      }
      objectValue["54"] = packedChildTypes;
    }
    if (object.ancestorTypes) {
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(item);
      }
      objectValue["55"] = packedAncestorTypes;
    }
    if (object.descendantTypes) {
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(item);
      }
      objectValue["56"] = packedDescendantTypes;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue !== undefined ? descriptionValue : null;
    const unpackedProperties: any[] = [];
    if (objectValue["50"] !== undefined) {
      for (const item of objectValue["50"]) {
        unpackedProperties.push(PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedTraits: any[] = [];
    if (objectValue["51"] !== undefined) {
      for (const item of objectValue["51"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const rootTypeValue = objectValue["52"];
    const unpackedRootType = rootTypeValue !== undefined ? Number(rootTypeValue) : null;
    const unpackedParentTypes: any[] = [];
    if (objectValue["53"] !== undefined) {
      for (const item of objectValue["53"]) {
        unpackedParentTypes.push(Number(item));
      }
    }
    const unpackedChildTypes: any[] = [];
    if (objectValue["54"] !== undefined) {
      for (const item of objectValue["54"]) {
        unpackedChildTypes.push(Number(item));
      }
    }
    const unpackedAncestorTypes: any[] = [];
    if (objectValue["55"] !== undefined) {
      for (const item of objectValue["55"]) {
        unpackedAncestorTypes.push(Number(item));
      }
    }
    const unpackedDescendantTypes: any[] = [];
    if (objectValue["56"] !== undefined) {
      for (const item of objectValue["56"]) {
        unpackedDescendantTypes.push(Number(item));
      }
    }
    return new NodeDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      properties: unpackedProperties,
      traits: unpackedTraits,
      rootType: unpackedRootType,
      parentTypes: unpackedParentTypes,
      childTypes: unpackedChildTypes,
      ancestorTypes: unpackedAncestorTypes,
      descendantTypes: unpackedDescendantTypes,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    return NodeDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = StructDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StructDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50007;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description !== null) {
      objectValue["36"] = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["50"] = packedProperties;
    }
    objectValue["60"] = object.isFrozen;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue !== undefined ? descriptionValue : null;
    const unpackedProperties: any[] = [];
    if (objectValue["50"] !== undefined) {
      for (const item of objectValue["50"]) {
        unpackedProperties.push(PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    return new StructDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      properties: unpackedProperties,
      isFrozen: objectValue["60"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    return StructDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = EnumDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: EnumDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50008;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description !== null) {
      objectValue["36"] = object.description;
    }
    if (object.options) {
      const packedOptions: any[] = [];
      for (const item of object.options) {
        packedOptions.push(item.toValue());
      }
      objectValue["50"] = packedOptions;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue !== undefined ? descriptionValue : null;
    const unpackedOptions: any[] = [];
    if (objectValue["50"] !== undefined) {
      for (const item of objectValue["50"]) {
        unpackedOptions.push(EnumOptionDefinition.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    return new EnumDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      options: unpackedOptions,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    return EnumDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = EnumOptionDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: EnumOptionDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50009;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description !== null) {
      objectValue["36"] = object.description;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumOptionDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue !== undefined ? descriptionValue : null;
    return new EnumOptionDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumOptionDefinition {
    return EnumOptionDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = PermissionDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PermissionDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50010;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["32"] = object.nodeType;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    return new PermissionDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      nodeType: Number(objectValue["32"]),
      icon: unpackedIcon,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    return PermissionDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = ConstantDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: ConstantDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50011;
    objectValue["31"] = object.name;
    objectValue["35"] = object.path;
    objectValue["40"] = object.value.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    return new ConstantDefinition({
      name: objectValue["31"],
      path: objectValue["35"],
      value: Value.fromValue(objectValue["40"], _session, _supergraph, _graph, _connection),
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    return ConstantDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50011 ==== */
