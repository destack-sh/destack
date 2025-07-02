import {
  CascadeAction,
  DefaultFactory,
  EdgeType,
  EnumType,
  NodeType,
  PrimitiveType,
  ScalarType,
  StructType,
  TraitType,
  TypeCardinality,
} from "@destack/language/core/builtin/common";
import type { ObjectDefinitionReference } from "@destack/language/core/builtin/relation";
import {
  ObjectDefinitionType,
  PropertyReference,
  PropertyReferenceType,
} from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { Icon } from "@destack/language/core/common/icon";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  StringConstraint,
  Type,
} from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import { STRUCT_CLASS_BY_TYPE, registerStructClass } from "@destack/language/registry";
import {
  CascadeActionProto,
  ConstantDefinitionProto,
  DefaultFactoryProto,
  EdgeTypeProto,
  EnumDefinitionProto,
  EnumTypeProto,
  NodeDefinitionProto,
  NodeTypeProto,
  OptionDefinitionProto,
  PermissionDefinitionProto,
  PrimitiveTypeProto,
  PropertyDefinitionProto,
  ScalarTypeProto,
  StructDefinitionProto,
  StructTypeProto,
  TraitDefinitionProto,
  TraitTypeProto,
  TypeCardinalityProto,
} from "@destack/proto";
import { assertNever, base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:102 ==== */
/**
 * Definition of a builtin Property.
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
   * The object that this property is defined on.
   */
  readonly object: ObjectDefinitionReference;

  /**
   * The original object that this property was defined on.
   */
  readonly originalObject: ObjectDefinitionReference;

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
   * PropertyDefinition.nodeIsExtensible
   */
  readonly nodeIsExtensible: boolean;

  /**
   * PropertyDefinition.nodeHasType
   */
  readonly nodeHasType: boolean;

  /**
   * PropertyDefinition.nodeHasSpace
   */
  readonly nodeHasSpace: boolean;

  /**
   * PropertyDefinition.nodeHasDefinition
   */
  readonly nodeHasDefinition: boolean;

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

  /**
   * PropertyDefinition.isReadonly
   */
  readonly isReadonly: boolean;

  /**
   * PropertyDefinition.isStatic
   */
  readonly isStatic: boolean;

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    object: ObjectDefinitionReference;
    originalObject: ObjectDefinitionReference;
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
    nodeIsExtensible: boolean;
    nodeHasType: boolean;
    nodeHasSpace: boolean;
    nodeHasDefinition: boolean;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isWired: boolean;
    isStored: boolean;
    isRepr: boolean;
    isHash: boolean;
    isEq: boolean;
    isManaged: boolean;
    isComputed: boolean;
    isReadonly: boolean;
    isStatic: boolean;
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
    let _object = options.object;
    if (_object === null) {
      throw new Error(`PropertyDefinition.object is required`);
    }
    this.object = _object;
    let _originalObject = options.originalObject;
    if (_originalObject === null) {
      throw new Error(`PropertyDefinition.originalObject is required`);
    }
    this.originalObject = _originalObject;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
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
    let _nodeIsExtensible = options.nodeIsExtensible;
    if (_nodeIsExtensible === null) {
      throw new Error(`PropertyDefinition.nodeIsExtensible is required`);
    }
    this.nodeIsExtensible = _nodeIsExtensible;
    let _nodeHasType = options.nodeHasType;
    if (_nodeHasType === null) {
      throw new Error(`PropertyDefinition.nodeHasType is required`);
    }
    this.nodeHasType = _nodeHasType;
    let _nodeHasSpace = options.nodeHasSpace;
    if (_nodeHasSpace === null) {
      throw new Error(`PropertyDefinition.nodeHasSpace is required`);
    }
    this.nodeHasSpace = _nodeHasSpace;
    let _nodeHasDefinition = options.nodeHasDefinition;
    if (_nodeHasDefinition === null) {
      throw new Error(`PropertyDefinition.nodeHasDefinition is required`);
    }
    this.nodeHasDefinition = _nodeHasDefinition;
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
    let _isReadonly = options.isReadonly;
    if (_isReadonly === null) {
      throw new Error(`PropertyDefinition.isReadonly is required`);
    }
    this.isReadonly = _isReadonly;
    let _isStatic = options.isStatic;
    if (_isStatic === null) {
      throw new Error(`PropertyDefinition.isStatic is required`);
    }
    this.isStatic = _isStatic;

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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (!this.object.equals(other.object)) {
      return false;
    }
    if (!this.originalObject.equals(other.originalObject)) {
      return false;
    }
    if (!(this.cardinality === other.cardinality)) {
      return false;
    }
    if (!(this.scalarType === other.scalarType)) {
      return false;
    }
    if (!(this.primitiveType === other.primitiveType)) {
      return false;
    }
    if (!(this.enumType === other.enumType)) {
      return false;
    }
    if (!(this.nodeType === other.nodeType)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (!(this.isRequired === other.isRequired)) {
      return false;
    }
    if (!(this.isUnique === other.isUnique)) {
      return false;
    }
    if (
      (this.defaultValue == null) !== (other.defaultValue == null) ||
      (this.defaultValue != null && !this.defaultValue.equals(other.defaultValue))
    ) {
      return false;
    }
    if (!(this.defaultFactory === other.defaultFactory)) {
      return false;
    }
    if (
      (this.collectionConstraint == null) !== (other.collectionConstraint == null) ||
      (this.collectionConstraint != null &&
        !this.collectionConstraint.equals(other.collectionConstraint))
    ) {
      return false;
    }
    if (
      (this.stringConstraint == null) !== (other.stringConstraint == null) ||
      (this.stringConstraint != null && !this.stringConstraint.equals(other.stringConstraint))
    ) {
      return false;
    }
    if (
      (this.numberConstraint == null) !== (other.numberConstraint == null) ||
      (this.numberConstraint != null && !this.numberConstraint.equals(other.numberConstraint))
    ) {
      return false;
    }
    if (
      (this.nodeConstraint == null) !== (other.nodeConstraint == null) ||
      (this.nodeConstraint != null && !this.nodeConstraint.equals(other.nodeConstraint))
    ) {
      return false;
    }
    if (!(this.nodeIsExtensible === other.nodeIsExtensible)) {
      return false;
    }
    if (!(this.nodeHasType === other.nodeHasType)) {
      return false;
    }
    if (!(this.nodeHasSpace === other.nodeHasSpace)) {
      return false;
    }
    if (!(this.nodeHasDefinition === other.nodeHasDefinition)) {
      return false;
    }
    if (!(this.edgeType === other.edgeType)) {
      return false;
    }
    if (!(this.cascade === other.cascade)) {
      return false;
    }
    if (!(this.isWired === other.isWired)) {
      return false;
    }
    if (!(this.isStored === other.isStored)) {
      return false;
    }
    if (!(this.isRepr === other.isRepr)) {
      return false;
    }
    if (!(this.isHash === other.isHash)) {
      return false;
    }
    if (!(this.isEq === other.isEq)) {
      return false;
    }
    if (!(this.isManaged === other.isManaged)) {
      return false;
    }
    if (!(this.isComputed === other.isComputed)) {
      return false;
    }
    if (!(this.isReadonly === other.isReadonly)) {
      return false;
    }
    if (!(this.isStatic === other.isStatic)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
      propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
      if (this.primitiveType !== null) {
        propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
      }
      if (this.enumType !== null) {
        propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
      }
      if (this.nodeType !== null) {
        propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      }
      if (this.structType !== null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.keyType !== null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      if (this.isRequired !== null) {
        propertyReprs.push(`isRequired=${this.isRequired}`);
      }
      if (this.isUnique !== null) {
        propertyReprs.push(`isUnique=${this.isUnique}`);
      }
      if (this.defaultValue !== null) {
        propertyReprs.push(`defaultValue=${this.defaultValue.repr()}`);
      }
      if (this.defaultFactory !== null) {
        propertyReprs.push(`defaultFactory=${DefaultFactory[this.defaultFactory]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PropertyDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    h = (h * 31 + this.object.hash()) & 0xffffffff;
    h = (h * 31 + this.originalObject.hash()) & 0xffffffff;
    h = (h * 31 + this.cardinality) & 0xffffffff;
    h = (h * 31 + this.scalarType) & 0xffffffff;
    if (this.primitiveType !== null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType !== null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeType !== null) {
      h = (h * 31 + this.nodeType) & 0xffffffff;
    }
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.keyType !== null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.isRequired !== null) {
      h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    }
    if (this.isUnique !== null) {
      h = (h * 31 + hashBool(this.isUnique)) & 0xffffffff;
    }
    if (this.defaultValue !== null) {
      h = (h * 31 + this.defaultValue.hash()) & 0xffffffff;
    }
    if (this.defaultFactory !== null) {
      h = (h * 31 + this.defaultFactory) & 0xffffffff;
    }
    if (this.collectionConstraint !== null) {
      h = (h * 31 + this.collectionConstraint.hash()) & 0xffffffff;
    }
    if (this.stringConstraint !== null) {
      h = (h * 31 + this.stringConstraint.hash()) & 0xffffffff;
    }
    if (this.numberConstraint !== null) {
      h = (h * 31 + this.numberConstraint.hash()) & 0xffffffff;
    }
    if (this.nodeConstraint !== null) {
      h = (h * 31 + this.nodeConstraint.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.nodeIsExtensible)) & 0xffffffff;
    h = (h * 31 + hashBool(this.nodeHasType)) & 0xffffffff;
    h = (h * 31 + hashBool(this.nodeHasSpace)) & 0xffffffff;
    h = (h * 31 + hashBool(this.nodeHasDefinition)) & 0xffffffff;
    if (this.edgeType !== null) {
      h = (h * 31 + this.edgeType) & 0xffffffff;
    }
    if (this.cascade !== null) {
      h = (h * 31 + this.cascade) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isWired)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isStored)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepr)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isHash)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isEq)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isManaged)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isComputed)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isReadonly)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isStatic)) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 102;
    objectValue["2"] = object.id;
    objectValue["31"] = object.name;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["36"] = object.description;
    }
    objectValue["37"] = object.object.toValue();
    objectValue["38"] = object.originalObject.toValue();
    objectValue["40"] = object.cardinality;
    objectValue["41"] = object.scalarType;
    if (object.primitiveType != null) {
      objectValue["42"] = object.primitiveType;
    }
    if (object.enumType != null) {
      objectValue["43"] = object.enumType;
    }
    if (object.nodeType != null) {
      objectValue["44"] = object.nodeType;
    }
    if (object.structType != null) {
      objectValue["46"] = object.structType;
    }
    if (object.keyType != null) {
      objectValue["48"] = object.keyType.toValue();
    }
    if (object.isRequired != null) {
      objectValue["50"] = object.isRequired;
    }
    if (object.isUnique != null) {
      objectValue["51"] = object.isUnique;
    }
    if (object.defaultValue != null) {
      objectValue["55"] = object.defaultValue.toValue();
    }
    if (object.defaultFactory != null) {
      objectValue["56"] = object.defaultFactory;
    }
    if (object.collectionConstraint != null) {
      objectValue["60"] = object.collectionConstraint.toValue();
    }
    if (object.stringConstraint != null) {
      objectValue["61"] = object.stringConstraint.toValue();
    }
    if (object.numberConstraint != null) {
      objectValue["62"] = object.numberConstraint.toValue();
    }
    if (object.nodeConstraint != null) {
      objectValue["63"] = object.nodeConstraint.toValue();
    }
    objectValue["73"] = object.nodeIsExtensible;
    objectValue["74"] = object.nodeHasType;
    objectValue["75"] = object.nodeHasSpace;
    objectValue["76"] = object.nodeHasDefinition;
    if (object.edgeType != null) {
      objectValue["77"] = object.edgeType;
    }
    if (object.cascade != null) {
      objectValue["78"] = object.cascade;
    }
    objectValue["80"] = object.isWired;
    objectValue["81"] = object.isStored;
    objectValue["82"] = object.isRepr;
    objectValue["83"] = object.isHash;
    objectValue["84"] = object.isEq;
    objectValue["85"] = object.isManaged;
    objectValue["86"] = object.isComputed;
    objectValue["87"] = object.isReadonly;
    objectValue["88"] = object.isStatic;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    const _ObjectDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.OBJECT_DEFINITION_REFERENCE
    ] as typeof ObjectDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const _NumberConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NUMBER_CONSTRAINT
    ] as typeof NumberConstraint;
    const _StringConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.STRING_CONSTRAINT
    ] as typeof StringConstraint;
    const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.COLLECTION_CONSTRAINT
    ] as typeof CollectionConstraint;
    const _NodeConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_CONSTRAINT
    ] as typeof NodeConstraint;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const primitiveTypeValue = objectValue["42"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["43"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["44"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["46"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const keyTypeValue = objectValue["48"];
    const unpackedKeyType =
      keyTypeValue != undefined
        ? _Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const isRequiredValue = objectValue["50"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const isUniqueValue = objectValue["51"];
    const unpackedIsUnique = isUniqueValue != undefined ? isUniqueValue : null;
    const defaultValueValue = objectValue["55"];
    const unpackedDefaultValue =
      defaultValueValue != undefined
        ? _Value.fromValue(defaultValueValue, _session, _supergraph, _graph, _connection)
        : null;
    const defaultFactoryValue = objectValue["56"];
    const unpackedDefaultFactory =
      defaultFactoryValue != undefined ? Number(defaultFactoryValue) : null;
    const collectionConstraintValue = objectValue["60"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? _CollectionConstraint.fromValue(
            collectionConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const stringConstraintValue = objectValue["61"];
    const unpackedStringConstraint =
      stringConstraintValue != undefined
        ? _StringConstraint.fromValue(
            stringConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const numberConstraintValue = objectValue["62"];
    const unpackedNumberConstraint =
      numberConstraintValue != undefined
        ? _NumberConstraint.fromValue(
            numberConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const nodeConstraintValue = objectValue["63"];
    const unpackedNodeConstraint =
      nodeConstraintValue != undefined
        ? _NodeConstraint.fromValue(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const edgeTypeValue = objectValue["77"];
    const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectValue["78"];
    const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : null;
    return new PropertyDefinition({
      id: Number(objectValue["2"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      object: _ObjectDefinitionReference.fromValue(
        objectValue["37"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      originalObject: _ObjectDefinitionReference.fromValue(
        objectValue["38"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      nodeIsExtensible: objectValue["73"],
      nodeHasType: objectValue["74"],
      nodeHasSpace: objectValue["75"],
      nodeHasDefinition: objectValue["76"],
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      isWired: objectValue["80"],
      isStored: objectValue["81"],
      isRepr: objectValue["82"],
      isHash: objectValue["83"],
      isEq: objectValue["84"],
      isManaged: objectValue["85"],
      isComputed: objectValue["86"],
      isReadonly: objectValue["87"],
      isStatic: objectValue["88"],
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
    return PropertyDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PropertyDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PropertyDefinition.__packProto__(this);
    }
    return this._proto as PropertyDefinitionProto;
  }

  static __packProto__(object: PropertyDefinition): PropertyDefinitionProto {
    const objectProto: Partial<PropertyDefinitionProto> = { metatype: 102 };
    objectProto.id = object.id;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    objectProto.object = object.object.toProto();
    objectProto.originalObject = object.originalObject.toProto();
    objectProto.cardinality = Number(object.cardinality) as TypeCardinalityProto;
    objectProto.scalarType = Number(object.scalarType) as ScalarTypeProto;
    if (object.primitiveType != null) {
      objectProto.primitiveType = Number(object.primitiveType) as PrimitiveTypeProto;
    }
    if (object.enumType != null) {
      objectProto.enumType = Number(object.enumType) as EnumTypeProto;
    }
    if (object.nodeType != null) {
      objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    if (object.keyType != null) {
      objectProto.keyType = object.keyType.toProto();
    }
    if (object.isRequired != null) {
      objectProto.isRequired = object.isRequired;
    }
    if (object.isUnique != null) {
      objectProto.isUnique = object.isUnique;
    }
    if (object.defaultValue != null) {
      objectProto.defaultValue = object.defaultValue.toProto();
    }
    if (object.defaultFactory != null) {
      objectProto.defaultFactory = Number(object.defaultFactory) as DefaultFactoryProto;
    }
    if (object.collectionConstraint != null) {
      objectProto.collectionConstraint = object.collectionConstraint.toProto();
    }
    if (object.stringConstraint != null) {
      objectProto.stringConstraint = object.stringConstraint.toProto();
    }
    if (object.numberConstraint != null) {
      objectProto.numberConstraint = object.numberConstraint.toProto();
    }
    if (object.nodeConstraint != null) {
      objectProto.nodeConstraint = object.nodeConstraint.toProto();
    }
    objectProto.nodeIsExtensible = object.nodeIsExtensible;
    objectProto.nodeHasType = object.nodeHasType;
    objectProto.nodeHasSpace = object.nodeHasSpace;
    objectProto.nodeHasDefinition = object.nodeHasDefinition;
    if (object.edgeType != null) {
      objectProto.edgeType = Number(object.edgeType) as EdgeTypeProto;
    }
    if (object.cascade != null) {
      objectProto.cascade = Number(object.cascade) as CascadeActionProto;
    }
    objectProto.isWired = object.isWired;
    objectProto.isStored = object.isStored;
    objectProto.isRepr = object.isRepr;
    objectProto.isHash = object.isHash;
    objectProto.isEq = object.isEq;
    objectProto.isManaged = object.isManaged;
    objectProto.isComputed = object.isComputed;
    objectProto.isReadonly = object.isReadonly;
    objectProto.isStatic = object.isStatic;
    return objectProto as PropertyDefinitionProto;
  }

  static __unpackProto__(
    objectProto: PropertyDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    const _ObjectDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.OBJECT_DEFINITION_REFERENCE
    ] as typeof ObjectDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const _NumberConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NUMBER_CONSTRAINT
    ] as typeof NumberConstraint;
    const _StringConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.STRING_CONSTRAINT
    ] as typeof StringConstraint;
    const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.COLLECTION_CONSTRAINT
    ] as typeof CollectionConstraint;
    const _NodeConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_CONSTRAINT
    ] as typeof NodeConstraint;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new PropertyDefinition({
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      object: _ObjectDefinitionReference.fromProto(
        objectProto.object!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      originalObject: _ObjectDefinitionReference.fromProto(
        objectProto.originalObject!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      cardinality: Number(objectProto.cardinality) as TypeCardinality,
      scalarType: Number(objectProto.scalarType) as ScalarType,
      primitiveType:
        objectProto.primitiveType != undefined
          ? (Number(objectProto.primitiveType) as PrimitiveType)
          : null,
      enumType:
        objectProto.enumType != undefined ? (Number(objectProto.enumType) as EnumType) : null,
      nodeType:
        objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      structType:
        objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      keyType:
        objectProto.keyType != undefined
          ? _Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      isUnique: objectProto.isUnique != undefined ? objectProto.isUnique : null,
      defaultValue:
        objectProto.defaultValue != undefined
          ? _Value.fromProto(objectProto.defaultValue!, _session, _supergraph, _graph, _connection)
          : null,
      defaultFactory:
        objectProto.defaultFactory != undefined
          ? (Number(objectProto.defaultFactory) as DefaultFactory)
          : null,
      collectionConstraint:
        objectProto.collectionConstraint != undefined
          ? _CollectionConstraint.fromProto(
              objectProto.collectionConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      stringConstraint:
        objectProto.stringConstraint != undefined
          ? _StringConstraint.fromProto(
              objectProto.stringConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      numberConstraint:
        objectProto.numberConstraint != undefined
          ? _NumberConstraint.fromProto(
              objectProto.numberConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      nodeConstraint:
        objectProto.nodeConstraint != undefined
          ? _NodeConstraint.fromProto(
              objectProto.nodeConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      nodeIsExtensible: objectProto.nodeIsExtensible,
      nodeHasType: objectProto.nodeHasType,
      nodeHasSpace: objectProto.nodeHasSpace,
      nodeHasDefinition: objectProto.nodeHasDefinition,
      edgeType:
        objectProto.edgeType != undefined ? (Number(objectProto.edgeType) as EdgeType) : null,
      cascade:
        objectProto.cascade != undefined ? (Number(objectProto.cascade) as CascadeAction) : null,
      isWired: objectProto.isWired,
      isStored: objectProto.isStored,
      isRepr: objectProto.isRepr,
      isHash: objectProto.isHash,
      isEq: objectProto.isEq,
      isManaged: objectProto.isManaged,
      isComputed: objectProto.isComputed,
      isReadonly: objectProto.isReadonly,
      isStatic: objectProto.isStatic,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PropertyDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    return PropertyDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PropertyDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PropertyDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  toRef(): PropertyReference {
    if (this.object.type === ObjectDefinitionType.BUILTIN_NODE) {
      return new PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        nodeType: this.object.nodeType,
        id: this.id,
      });
    } else if (this.object.type === ObjectDefinitionType.BUILTIN_STRUCT) {
      return new PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        structType: this.object.structType,
        id: this.id,
      });
    } else if (this.object.type === ObjectDefinitionType.BUILTIN_TRAIT) {
      return new PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        traitType: this.object.traitType,
        id: this.id,
      });
    } else if (
      this.object.type === ObjectDefinitionType.CUSTOM_NODE ||
      this.object.type === ObjectDefinitionType.CUSTOM_STRUCT ||
      this.object.type === ObjectDefinitionType.CUSTOM_TRAIT
    ) {
      throw new Error(`${this.repr()} cannot be associated with a custom object`);
    } else {
      assertNever(this.object.type);
    }
  }

  eq(value: any): Condition {
    if (value === null) {
      return Condition.of(this, ConditionalType.NOT_EXISTS);
    }
    return Condition.of(this, ConditionalType.EQUALS, value);
  }

  neq(value: any): Condition {
    if (value === null) {
      return Condition.of(this, ConditionalType.EXISTS);
    }
    return Condition.of(this, ConditionalType.NOT_EQUALS, value);
  }

  gt(value: any): Condition {
    return Condition.of(this, ConditionalType.GREATER_THAN, value);
  }

  gte(value: any): Condition {
    return Condition.of(this, ConditionalType.GREATER_THAN_OR_EQUALS, value);
  }

  lt(value: any): Condition {
    return Condition.of(this, ConditionalType.LESS_THAN, value);
  }

  lte(value: any): Condition {
    return Condition.of(this, ConditionalType.LESS_THAN_OR_EQUALS, value);
  }

  startsWith(value: string): Condition {
    return Condition.of(this, ConditionalType.STARTS_WITH, value);
  }

  endsWith(value: string): Condition {
    return Condition.of(this, ConditionalType.ENDS_WITH, value);
  }

  in(...values: any[]): Condition {
    return Condition.of(this, ConditionalType.IN, values);
  }

  notIn(...values: any[]): Condition {
    return Condition.of(this, ConditionalType.NOT_IN, values);
  }

  exists(): Condition {
    return Condition.of(this, ConditionalType.EXISTS);
  }

  isNotNone(): Condition {
    return Condition.of(this, ConditionalType.EXISTS);
  }

  notExists(): Condition {
    return Condition.of(this, ConditionalType.NOT_EXISTS);
  }

  isNone(): Condition {
    return Condition.of(this, ConditionalType.NOT_EXISTS);
  }

  asc(): Sort {
    return Sort.of(this, SortType.ASCENDING);
  }

  desc(): Sort {
    return Sort.of(this, SortType.DESCENDING);
  }

  _type: Type | null = null;

  toType(): Type {
    if (this._type === null) {
      const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
      this._type = new _Type({
        cardinality: this.cardinality,
        scalarType: this.scalarType,
        primitiveType: this.primitiveType,
        enumType: this.enumType,
        nodeType: this.nodeType,
        structType: this.structType,
        keyType: this.keyType,
        isRequired: this.isRequired,
        defaultValue: this.defaultValue,
        defaultFactory: this.defaultFactory,
        collectionConstraint: this.collectionConstraint,
        stringConstraint: this.stringConstraint,
        numberConstraint: this.numberConstraint,
        nodeConstraint: this.nodeConstraint,
      });
    }
    return this._type;
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PROPERTY_DEFINITION, PropertyDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:102 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:103 ==== */
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
   * Traits directly and indirectly inherited by this trait.
   */
  readonly traits: Array<TraitType>;

  /**
   * Traits directly inherited by this trait.
   */
  readonly baseTraits: Array<TraitType>;

  constructor(options: {
    id: number;
    type: TraitType;
    name: string;
    alias: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    traits?: Array<TraitType>;
    baseTraits?: Array<TraitType>;
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
      _properties = [];
    }
    this.properties = _properties;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      _traits = [];
    }
    this.traits = _traits;
    let _baseTraits = options.baseTraits ?? null;
    if (_baseTraits === null) {
      _baseTraits = [];
    }
    this.baseTraits = _baseTraits;

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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.alias === other.alias)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.properties.length !== other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.traits.length !== other.traits.length) {
      return false;
    }
    for (let i = 0; i < this.traits.length; i++) {
      if (!(this.traits[i] === other.traits[i])) {
        return false;
      }
    }
    if (this.baseTraits.length !== other.baseTraits.length) {
      return false;
    }
    for (let i = 0; i < this.baseTraits.length; i++) {
      if (!(this.baseTraits[i] === other.baseTraits[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${TraitType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      propertyReprs.push(`alias=${this.alias}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<TraitDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.alias)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.traits && this.traits.length > 0) {
      for (const _item of this.traits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.baseTraits && this.baseTraits.length > 0) {
      for (const _item of this.baseTraits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 103;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["32"] = object.alias;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["36"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["40"] = packedProperties;
    }
    if (object.traits.length > 0) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectValue["51"] = packedTraits;
    }
    if (object.baseTraits.length > 0) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(item);
      }
      objectValue["52"] = packedBaseTraits;
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
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedProperties: any[] = [];
    if (objectValue["40"] != undefined) {
      for (const item of objectValue["40"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTraits: any[] = [];
    if (objectValue["51"] != undefined) {
      for (const item of objectValue["51"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectValue["52"] != undefined) {
      for (const item of objectValue["52"]) {
        unpackedBaseTraits.push(Number(item));
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
      baseTraits: unpackedBaseTraits,
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

  toProto(): TraitDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = TraitDefinition.__packProto__(this);
    }
    return this._proto as TraitDefinitionProto;
  }

  static __packProto__(object: TraitDefinition): TraitDefinitionProto {
    const objectProto: Partial<TraitDefinitionProto> = { metatype: 103 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as TraitTypeProto;
    objectProto.name = object.name;
    objectProto.alias = object.alias;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.traits = packedTraits;
    }
    if (object.baseTraits) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.baseTraits = packedBaseTraits;
    }
    return objectProto as TraitDefinitionProto;
  }

  static __unpackProto__(
    objectProto: TraitDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTraits: any[] = [];
    if (objectProto.traits) {
      for (const item of objectProto.traits) {
        unpackedTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectProto.baseTraits) {
      for (const item of objectProto.baseTraits) {
        unpackedBaseTraits.push(Number(item) as TraitType);
      }
    }
    return new TraitDefinition({
      id: Number(objectProto.id),
      type: Number(objectProto.type) as TraitType,
      name: objectProto.name,
      alias: objectProto.alias,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      properties: unpackedProperties,
      traits: unpackedTraits,
      baseTraits: unpackedBaseTraits,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: TraitDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    return TraitDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): TraitDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TraitDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TRAIT_DEFINITION, TraitDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:104 ==== */
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
   * NodeDefinition.isAbstract
   */
  readonly isAbstract: boolean;

  /**
   * NodeDefinition.isGlobal
   */
  readonly isGlobal: boolean;

  /**
   * NodeDefinition.isSpatial
   */
  readonly isSpatial: boolean;

  /**
   * NodeDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * The base type this Node extends (directly).
   */
  readonly baseType: NodeType | null;

  /**
   * Nodes that this Node extends (directly and indirectly).
   */
  readonly extends: Array<NodeType>;

  /**
   * Nodes that extend this Node type (directly).
   */
  readonly extendedBy: Array<NodeType>;

  /**
   * Nodes that inherit this Node type (directly and indirectly).
   */
  readonly inheritedBy: Array<NodeType>;

  /**
   * Traits directly inherited by this Node (directly).
   */
  readonly baseTraits: Array<TraitType>;

  /**
   * Traits directly and indirectly inherited by this Node (directly and indirectly).
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
    isAbstract: boolean;
    isGlobal: boolean;
    isSpatial: boolean;
    properties?: Array<PropertyDefinition>;
    baseType?: NodeType | null;
    extends?: Array<NodeType>;
    extendedBy?: Array<NodeType>;
    inheritedBy?: Array<NodeType>;
    baseTraits?: Array<TraitType>;
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
    let _isAbstract = options.isAbstract;
    if (_isAbstract === null) {
      throw new Error(`NodeDefinition.isAbstract is required`);
    }
    this.isAbstract = _isAbstract;
    let _isGlobal = options.isGlobal;
    if (_isGlobal === null) {
      throw new Error(`NodeDefinition.isGlobal is required`);
    }
    this.isGlobal = _isGlobal;
    let _isSpatial = options.isSpatial;
    if (_isSpatial === null) {
      throw new Error(`NodeDefinition.isSpatial is required`);
    }
    this.isSpatial = _isSpatial;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _extends = options.extends ?? null;
    if (_extends === null) {
      _extends = [];
    }
    this.extends = _extends;
    let _extendedBy = options.extendedBy ?? null;
    if (_extendedBy === null) {
      _extendedBy = [];
    }
    this.extendedBy = _extendedBy;
    let _inheritedBy = options.inheritedBy ?? null;
    if (_inheritedBy === null) {
      _inheritedBy = [];
    }
    this.inheritedBy = _inheritedBy;
    let _baseTraits = options.baseTraits ?? null;
    if (_baseTraits === null) {
      _baseTraits = [];
    }
    this.baseTraits = _baseTraits;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      _traits = [];
    }
    this.traits = _traits;
    let _rootType = options.rootType ?? null;
    this.rootType = _rootType;
    let _parentTypes = options.parentTypes ?? null;
    if (_parentTypes === null) {
      _parentTypes = [];
    }
    this.parentTypes = _parentTypes;
    let _childTypes = options.childTypes ?? null;
    if (_childTypes === null) {
      _childTypes = [];
    }
    this.childTypes = _childTypes;
    let _ancestorTypes = options.ancestorTypes ?? null;
    if (_ancestorTypes === null) {
      _ancestorTypes = [];
    }
    this.ancestorTypes = _ancestorTypes;
    let _descendantTypes = options.descendantTypes ?? null;
    if (_descendantTypes === null) {
      _descendantTypes = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (!(this.isAbstract === other.isAbstract)) {
      return false;
    }
    if (!(this.isGlobal === other.isGlobal)) {
      return false;
    }
    if (!(this.isSpatial === other.isSpatial)) {
      return false;
    }
    if (this.properties.length !== other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (!(this.baseType === other.baseType)) {
      return false;
    }
    if (this.extends.length !== other.extends.length) {
      return false;
    }
    for (let i = 0; i < this.extends.length; i++) {
      if (!(this.extends[i] === other.extends[i])) {
        return false;
      }
    }
    if (this.extendedBy.length !== other.extendedBy.length) {
      return false;
    }
    for (let i = 0; i < this.extendedBy.length; i++) {
      if (!(this.extendedBy[i] === other.extendedBy[i])) {
        return false;
      }
    }
    if (this.inheritedBy.length !== other.inheritedBy.length) {
      return false;
    }
    for (let i = 0; i < this.inheritedBy.length; i++) {
      if (!(this.inheritedBy[i] === other.inheritedBy[i])) {
        return false;
      }
    }
    if (this.baseTraits.length !== other.baseTraits.length) {
      return false;
    }
    for (let i = 0; i < this.baseTraits.length; i++) {
      if (!(this.baseTraits[i] === other.baseTraits[i])) {
        return false;
      }
    }
    if (this.traits.length !== other.traits.length) {
      return false;
    }
    for (let i = 0; i < this.traits.length; i++) {
      if (!(this.traits[i] === other.traits[i])) {
        return false;
      }
    }
    if (!(this.rootType === other.rootType)) {
      return false;
    }
    if (this.parentTypes.length !== other.parentTypes.length) {
      return false;
    }
    for (let i = 0; i < this.parentTypes.length; i++) {
      if (!(this.parentTypes[i] === other.parentTypes[i])) {
        return false;
      }
    }
    if (this.childTypes.length !== other.childTypes.length) {
      return false;
    }
    for (let i = 0; i < this.childTypes.length; i++) {
      if (!(this.childTypes[i] === other.childTypes[i])) {
        return false;
      }
    }
    if (this.ancestorTypes.length !== other.ancestorTypes.length) {
      return false;
    }
    for (let i = 0; i < this.ancestorTypes.length; i++) {
      if (!(this.ancestorTypes[i] === other.ancestorTypes[i])) {
        return false;
      }
    }
    if (this.descendantTypes.length !== other.descendantTypes.length) {
      return false;
    }
    for (let i = 0; i < this.descendantTypes.length; i++) {
      if (!(this.descendantTypes[i] === other.descendantTypes[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${NodeType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      propertyReprs.push(`isAbstract=${this.isAbstract}`);
      propertyReprs.push(`isGlobal=${this.isGlobal}`);
      propertyReprs.push(`isSpatial=${this.isSpatial}`);
      // @ts-expect-error(readonly)
      this._repr = `<NodeDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isAbstract)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isGlobal)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isSpatial)) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType) & 0xffffffff;
    }
    if (this.extends && this.extends.length > 0) {
      for (const _item of this.extends) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.extendedBy && this.extendedBy.length > 0) {
      for (const _item of this.extendedBy) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.inheritedBy && this.inheritedBy.length > 0) {
      for (const _item of this.inheritedBy) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.baseTraits && this.baseTraits.length > 0) {
      for (const _item of this.baseTraits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.traits && this.traits.length > 0) {
      for (const _item of this.traits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.rootType !== null) {
      h = (h * 31 + this.rootType) & 0xffffffff;
    }
    if (this.parentTypes && this.parentTypes.length > 0) {
      for (const _item of this.parentTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.childTypes && this.childTypes.length > 0) {
      for (const _item of this.childTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.ancestorTypes && this.ancestorTypes.length > 0) {
      for (const _item of this.ancestorTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.descendantTypes && this.descendantTypes.length > 0) {
      for (const _item of this.descendantTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 104;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["36"] = object.description;
    }
    objectValue["37"] = object.isAbstract;
    objectValue["38"] = object.isGlobal;
    objectValue["39"] = object.isSpatial;
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["40"] = packedProperties;
    }
    if (object.baseType != null) {
      objectValue["50"] = object.baseType;
    }
    if (object.extends.length > 0) {
      const packedExtends: any[] = [];
      for (const item of object.extends) {
        packedExtends.push(item);
      }
      objectValue["51"] = packedExtends;
    }
    if (object.extendedBy.length > 0) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectValue["52"] = packedExtendedBy;
    }
    if (object.inheritedBy.length > 0) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectValue["53"] = packedInheritedBy;
    }
    if (object.baseTraits.length > 0) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(item);
      }
      objectValue["55"] = packedBaseTraits;
    }
    if (object.traits.length > 0) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectValue["56"] = packedTraits;
    }
    if (object.rootType != null) {
      objectValue["60"] = object.rootType;
    }
    if (object.parentTypes.length > 0) {
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(item);
      }
      objectValue["61"] = packedParentTypes;
    }
    if (object.childTypes.length > 0) {
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(item);
      }
      objectValue["62"] = packedChildTypes;
    }
    if (object.ancestorTypes.length > 0) {
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(item);
      }
      objectValue["63"] = packedAncestorTypes;
    }
    if (object.descendantTypes.length > 0) {
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(item);
      }
      objectValue["64"] = packedDescendantTypes;
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
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedProperties: any[] = [];
    if (objectValue["40"] != undefined) {
      for (const item of objectValue["40"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const baseTypeValue = objectValue["50"];
    const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : null;
    const unpackedExtends: any[] = [];
    if (objectValue["51"] != undefined) {
      for (const item of objectValue["51"]) {
        unpackedExtends.push(Number(item));
      }
    }
    const unpackedExtendedBy: any[] = [];
    if (objectValue["52"] != undefined) {
      for (const item of objectValue["52"]) {
        unpackedExtendedBy.push(Number(item));
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectValue["53"] != undefined) {
      for (const item of objectValue["53"]) {
        unpackedInheritedBy.push(Number(item));
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectValue["55"] != undefined) {
      for (const item of objectValue["55"]) {
        unpackedBaseTraits.push(Number(item));
      }
    }
    const unpackedTraits: any[] = [];
    if (objectValue["56"] != undefined) {
      for (const item of objectValue["56"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const rootTypeValue = objectValue["60"];
    const unpackedRootType = rootTypeValue != undefined ? Number(rootTypeValue) : null;
    const unpackedParentTypes: any[] = [];
    if (objectValue["61"] != undefined) {
      for (const item of objectValue["61"]) {
        unpackedParentTypes.push(Number(item));
      }
    }
    const unpackedChildTypes: any[] = [];
    if (objectValue["62"] != undefined) {
      for (const item of objectValue["62"]) {
        unpackedChildTypes.push(Number(item));
      }
    }
    const unpackedAncestorTypes: any[] = [];
    if (objectValue["63"] != undefined) {
      for (const item of objectValue["63"]) {
        unpackedAncestorTypes.push(Number(item));
      }
    }
    const unpackedDescendantTypes: any[] = [];
    if (objectValue["64"] != undefined) {
      for (const item of objectValue["64"]) {
        unpackedDescendantTypes.push(Number(item));
      }
    }
    return new NodeDefinition({
      id: Number(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      description: unpackedDescription,
      isAbstract: objectValue["37"],
      isGlobal: objectValue["38"],
      isSpatial: objectValue["39"],
      properties: unpackedProperties,
      baseType: unpackedBaseType,
      extends: unpackedExtends,
      extendedBy: unpackedExtendedBy,
      inheritedBy: unpackedInheritedBy,
      baseTraits: unpackedBaseTraits,
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

  toProto(): NodeDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NodeDefinition.__packProto__(this);
    }
    return this._proto as NodeDefinitionProto;
  }

  static __packProto__(object: NodeDefinition): NodeDefinitionProto {
    const objectProto: Partial<NodeDefinitionProto> = { metatype: 104 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as NodeTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    objectProto.isAbstract = object.isAbstract;
    objectProto.isGlobal = object.isGlobal;
    objectProto.isSpatial = object.isSpatial;
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.baseType != null) {
      objectProto.baseType = Number(object.baseType) as NodeTypeProto;
    }
    if (object.extends) {
      const packedExtends: any[] = [];
      for (const item of object.extends) {
        packedExtends.push(Number(item) as NodeTypeProto);
      }
      objectProto.extends = packedExtends;
    }
    if (object.extendedBy) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(Number(item) as NodeTypeProto);
      }
      objectProto.extendedBy = packedExtendedBy;
    }
    if (object.inheritedBy) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(Number(item) as NodeTypeProto);
      }
      objectProto.inheritedBy = packedInheritedBy;
    }
    if (object.baseTraits) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.baseTraits = packedBaseTraits;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.traits = packedTraits;
    }
    if (object.rootType != null) {
      objectProto.rootType = Number(object.rootType) as NodeTypeProto;
    }
    if (object.parentTypes) {
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.parentTypes = packedParentTypes;
    }
    if (object.childTypes) {
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.childTypes = packedChildTypes;
    }
    if (object.ancestorTypes) {
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.ancestorTypes = packedAncestorTypes;
    }
    if (object.descendantTypes) {
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.descendantTypes = packedDescendantTypes;
    }
    return objectProto as NodeDefinitionProto;
  }

  static __unpackProto__(
    objectProto: NodeDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedExtends: any[] = [];
    if (objectProto.extends) {
      for (const item of objectProto.extends) {
        unpackedExtends.push(Number(item) as NodeType);
      }
    }
    const unpackedExtendedBy: any[] = [];
    if (objectProto.extendedBy) {
      for (const item of objectProto.extendedBy) {
        unpackedExtendedBy.push(Number(item) as NodeType);
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectProto.inheritedBy) {
      for (const item of objectProto.inheritedBy) {
        unpackedInheritedBy.push(Number(item) as NodeType);
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectProto.baseTraits) {
      for (const item of objectProto.baseTraits) {
        unpackedBaseTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedTraits: any[] = [];
    if (objectProto.traits) {
      for (const item of objectProto.traits) {
        unpackedTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedParentTypes: any[] = [];
    if (objectProto.parentTypes) {
      for (const item of objectProto.parentTypes) {
        unpackedParentTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedChildTypes: any[] = [];
    if (objectProto.childTypes) {
      for (const item of objectProto.childTypes) {
        unpackedChildTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedAncestorTypes: any[] = [];
    if (objectProto.ancestorTypes) {
      for (const item of objectProto.ancestorTypes) {
        unpackedAncestorTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedDescendantTypes: any[] = [];
    if (objectProto.descendantTypes) {
      for (const item of objectProto.descendantTypes) {
        unpackedDescendantTypes.push(Number(item) as NodeType);
      }
    }
    return new NodeDefinition({
      id: Number(objectProto.id),
      type: Number(objectProto.type) as NodeType,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      isAbstract: objectProto.isAbstract,
      isGlobal: objectProto.isGlobal,
      isSpatial: objectProto.isSpatial,
      properties: unpackedProperties,
      baseType:
        objectProto.baseType != undefined ? (Number(objectProto.baseType) as NodeType) : null,
      extends: unpackedExtends,
      extendedBy: unpackedExtendedBy,
      inheritedBy: unpackedInheritedBy,
      baseTraits: unpackedBaseTraits,
      traits: unpackedTraits,
      rootType:
        objectProto.rootType != undefined ? (Number(objectProto.rootType) as NodeType) : null,
      parentTypes: unpackedParentTypes,
      childTypes: unpackedChildTypes,
      ancestorTypes: unpackedAncestorTypes,
      descendantTypes: unpackedDescendantTypes,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: NodeDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    return NodeDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): NodeDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = NodeDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_DEFINITION, NodeDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:104 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:105 ==== */
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
      _properties = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.properties.length !== other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (!(this.isFrozen === other.isFrozen)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${StructType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<StructDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashBool(this.isFrozen)) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 105;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["36"] = object.description;
    }
    if (object.properties.length > 0) {
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
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedProperties: any[] = [];
    if (objectValue["50"] != undefined) {
      for (const item of objectValue["50"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
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
    return StructDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): StructDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StructDefinition.__packProto__(this);
    }
    return this._proto as StructDefinitionProto;
  }

  static __packProto__(object: StructDefinition): StructDefinitionProto {
    const objectProto: Partial<StructDefinitionProto> = { metatype: 105 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as StructTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    objectProto.isFrozen = object.isFrozen;
    return objectProto as StructDefinitionProto;
  }

  static __unpackProto__(
    objectProto: StructDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new StructDefinition({
      id: Number(objectProto.id),
      type: Number(objectProto.type) as StructType,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      properties: unpackedProperties,
      isFrozen: objectProto.isFrozen,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StructDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    return StructDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): StructDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StructDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STRUCT_DEFINITION, StructDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:105 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:106 ==== */
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
  readonly options: Array<OptionDefinition>;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    options?: Array<OptionDefinition>;
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
      _options = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.options.length !== other.options.length) {
      return false;
    }
    for (let i = 0; i < this.options.length; i++) {
      if (!this.options[i].equals(other.options[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<EnumDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.options && this.options.length > 0) {
      for (const _item of this.options) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 106;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["36"] = object.description;
    }
    if (object.options.length > 0) {
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
    const _OptionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.OPTION_DEFINITION
    ] as typeof OptionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedOptions: any[] = [];
    if (objectValue["50"] != undefined) {
      for (const item of objectValue["50"]) {
        unpackedOptions.push(
          _OptionDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
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

  toProto(): EnumDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = EnumDefinition.__packProto__(this);
    }
    return this._proto as EnumDefinitionProto;
  }

  static __packProto__(object: EnumDefinition): EnumDefinitionProto {
    const objectProto: Partial<EnumDefinitionProto> = { metatype: 106 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.options) {
      const packedOptions: any[] = [];
      for (const item of object.options) {
        packedOptions.push(item.toProto());
      }
      objectProto.options = packedOptions;
    }
    return objectProto as EnumDefinitionProto;
  }

  static __unpackProto__(
    objectProto: EnumDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    const _OptionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.OPTION_DEFINITION
    ] as typeof OptionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedOptions: any[] = [];
    if (objectProto.options) {
      for (const item of objectProto.options) {
        unpackedOptions.push(
          _OptionDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new EnumDefinition({
      id: Number(objectProto.id),
      type: Number(objectProto.type) as EnumType,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      options: unpackedOptions,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: EnumDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    return EnumDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): EnumDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EnumDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ENUM_DEFINITION, EnumDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:106 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:107 ==== */
/**
 * Definition of a builtin Enum Option.
 */
export class OptionDefinition extends StructFrozen {
  static metatype: StructType = StructType.OPTION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * OptionDefinition.id
   */
  readonly id: number;

  /**
   * OptionDefinition.type
   */
  readonly type: EnumType;

  /**
   * OptionDefinition.name
   */
  readonly name: string;

  /**
   * OptionDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * OptionDefinition.description
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
      throw new Error(`OptionDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`OptionDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`OptionDefinition.name is required`);
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<OptionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = OptionDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: OptionDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 107;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.description != null) {
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
  ): OptionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new OptionDefinition({
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
  ): OptionDefinition {
    return OptionDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): OptionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = OptionDefinition.__packProto__(this);
    }
    return this._proto as OptionDefinitionProto;
  }

  static __packProto__(object: OptionDefinition): OptionDefinitionProto {
    const objectProto: Partial<OptionDefinitionProto> = { metatype: 107 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    return objectProto as OptionDefinitionProto;
  }

  static __unpackProto__(
    objectProto: OptionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new OptionDefinition({
      id: Number(objectProto.id),
      type: Number(objectProto.type) as EnumType,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: OptionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    return OptionDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): OptionDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = OptionDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.OPTION_DEFINITION, OptionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:107 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:108 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.nodeType === other.nodeType)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      // @ts-expect-error(readonly)
      this._repr = `<PermissionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + this.nodeType) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 108;
    objectValue["2"] = object.id;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["32"] = object.nodeType;
    if (object.icon != null) {
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
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
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
    return PermissionDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PermissionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PermissionDefinition.__packProto__(this);
    }
    return this._proto as PermissionDefinitionProto;
  }

  static __packProto__(object: PermissionDefinition): PermissionDefinitionProto {
    const objectProto: Partial<PermissionDefinitionProto> = { metatype: 108 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    return objectProto as PermissionDefinitionProto;
  }

  static __unpackProto__(
    objectProto: PermissionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new PermissionDefinition({
      id: Number(objectProto.id),
      type: Number(objectProto.type) as EnumType,
      name: objectProto.name,
      nodeType: Number(objectProto.nodeType) as NodeType,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PermissionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    return PermissionDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PermissionDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PermissionDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PERMISSION_DEFINITION, PermissionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:108 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:109 ==== */
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
   * ConstantDefinition.description
   */
  readonly description: string | null;

  /**
   * ConstantDefinition.value
   */
  readonly value: Value;

  /**
   * ConstantDefinition.isDeferred
   */
  readonly isDeferred: boolean;

  constructor(options: {
    name: string;
    description?: string | null;
    value: Value;
    isDeferred: boolean;
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
    let _description = options.description ?? null;
    this.description = _description;
    let _value = options.value;
    if (_value === null) {
      throw new Error(`ConstantDefinition.value is required`);
    }
    this.value = _value;
    let _isDeferred = options.isDeferred;
    if (_isDeferred === null) {
      throw new Error(`ConstantDefinition.isDeferred is required`);
    }
    this.isDeferred = _isDeferred;

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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (!this.value.equals(other.value)) {
      return false;
    }
    if (!(this.isDeferred === other.isDeferred)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ConstantDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    h = (h * 31 + this.value.hash()) & 0xffffffff;
    h = (h * 31 + hashBool(this.isDeferred)) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 109;
    objectValue["31"] = object.name;
    if (object.description != null) {
      objectValue["36"] = object.description;
    }
    objectValue["40"] = object.value.toValue();
    objectValue["50"] = object.isDeferred;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const descriptionValue = objectValue["36"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new ConstantDefinition({
      name: objectValue["31"],
      description: unpackedDescription,
      value: _Value.fromValue(objectValue["40"], _session, _supergraph, _graph, _connection),
      isDeferred: objectValue["50"],
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
    return ConstantDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): ConstantDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ConstantDefinition.__packProto__(this);
    }
    return this._proto as ConstantDefinitionProto;
  }

  static __packProto__(object: ConstantDefinition): ConstantDefinitionProto {
    const objectProto: Partial<ConstantDefinitionProto> = { metatype: 109 };
    objectProto.name = object.name;
    if (object.description != null) {
      objectProto.description = object.description;
    }
    objectProto.value = object.value.toProto();
    objectProto.isDeferred = object.isDeferred;
    return objectProto as ConstantDefinitionProto;
  }

  static __unpackProto__(
    objectProto: ConstantDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    return new ConstantDefinition({
      name: objectProto.name,
      description: objectProto.description != undefined ? objectProto.description : null,
      value: _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection),
      isDeferred: objectProto.isDeferred,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ConstantDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    return ConstantDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): ConstantDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ConstantDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CONSTANT_DEFINITION, ConstantDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:109 ==== */
