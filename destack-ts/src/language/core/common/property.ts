import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  CascadeAction,
  EdgeType,
  EnumType,
  NodeType,
  PrimitiveType,
  ScalarType,
  StructType,
  TypeCardinality,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import type {
  CustomEntityDefinition,
  CustomTraitDefinition,
} from "@destack/language/core/builtin/entity";
import { Entity } from "@destack/language/core/builtin/entity";
import type { CustomEventDefinition } from "@destack/language/core/builtin/event";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  HasIcon,
  HasName,
  IsCustomizable,
  IsDeletable,
  IsExtensible,
  IsSourceable,
  IsSpatial,
  IsSubject,
  IsTaggable,
} from "@destack/language/core/builtin/trait";
import type { CustomEnumDefinition } from "@destack/language/core/common/enum";
import type { Icon } from "@destack/language/core/common/icon";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type { CustomStructDefinition } from "@destack/language/core/common/struct";
import type {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  StringConstraint,
  Type,
} from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import {
  CascadeActionProto,
  CustomPropertyProto,
  CustomPropertyTypeProto,
  EdgeTypeProto,
  EnumTypeProto,
  NodeTypeProto,
  PrimitiveTypeProto,
  ScalarTypeProto,
  StructTypeProto,
  TypeCardinalityProto,
  ValueFactoryProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:320 ==== */
/**
 * A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
 */
export class CustomProperty
  extends Entity
  implements IsSpatial, HasName, HasIcon, IsTaggable, IsDeletable, IsSourceable
{
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY;

  /**
   * CustomProperty.parent
   */
  get parent(): (Node & IsCustomizable) | (Node & IsExtensible) | CustomProperty | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | (Node & IsCustomizable)
        | (Node & IsExtensible)
        | CustomProperty
        | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * CustomProperty.type
   */
  type: CustomPropertyType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /**
   * CustomProperty.cardinality
   */
  cardinality: TypeCardinality;

  /**
   * CustomProperty.scalarType
   */
  scalarType: ScalarType;

  /**
   * CustomProperty.primitiveType
   */
  primitiveType: PrimitiveType | null;

  /**
   * CustomProperty.enumType
   */
  enumType: EnumType | null;

  /**
   * CustomProperty.nodeType
   */
  nodeType: NodeType | null;

  /**
   * CustomProperty.structType
   */
  structType: StructType | null;

  /**
   * CustomProperty.definition
   */
  get definition():
    | CustomEntityDefinition
    | CustomEventDefinition
    | CustomEnumDefinition
    | CustomStructDefinition
    | CustomTraitDefinition
    | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | CustomEnumDefinition
        | CustomStructDefinition
        | CustomTraitDefinition
        | null;
    }
    return null;
  }
  set definition(
    node:
      | CustomEntityDefinition
      | CustomEventDefinition
      | CustomEnumDefinition
      | CustomStructDefinition
      | CustomTraitDefinition
      | null,
  ) {
    if (node === null) {
      this.definitionPtr = null;
    } else {
      this.definitionPtr = node.toRef();
    }
  }
  definitionPtr: NodeReference | null;

  /**
   * CustomProperty.keyType
   */
  keyType: Type | null;

  /**
   * CustomProperty.value
   */
  value: Value | null;

  /**
   * CustomProperty.valueFactory
   */
  valueFactory: ValueFactory | null;

  /**
   * CustomProperty.collectionConstraint
   */
  collectionConstraint: CollectionConstraint | null;

  /**
   * CustomProperty.stringConstraint
   */
  stringConstraint: StringConstraint | null;

  /**
   * CustomProperty.numberConstraint
   */
  numberConstraint: NumberConstraint | null;

  /**
   * CustomProperty.nodeConstraint
   */
  nodeConstraint: NodeConstraint | null;

  /**
   * CustomProperty.edgeType
   */
  edgeType: EdgeType | null;

  /**
   * CustomProperty.cascade
   */
  cascade: CascadeAction | null;

  /**
   * CustomProperty.isRequired
   */
  isRequired: boolean | null;

  /**
   * CustomProperty.isUnique
   */
  isUnique: boolean | null;

  /**
   * CustomProperty.isComputed
   */
  isComputed: boolean | null;

  /**
   * CustomProperty.isReadonly
   */
  isReadonly: boolean | null;

  /**
   * CustomProperty.isStatic
   */
  isStatic: boolean | null;

  /**
   * IsSourceable.source
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?:
      | (Node & IsCustomizable)
      | (Node & IsExtensible)
      | CustomProperty
      | NodeReference
      | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type?: CustomPropertyType;
    name: string;
    icon?: Icon | null;
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    structType?: StructType | null;
    definition?:
      | CustomEntityDefinition
      | CustomEventDefinition
      | CustomEnumDefinition
      | CustomStructDefinition
      | CustomTraitDefinition
      | NodeReference
      | null;
    keyType?: Type | null;
    value?: Value | null;
    valueFactory?: ValueFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isRequired?: boolean | null;
    isUnique?: boolean | null;
    isComputed?: boolean | null;
    isReadonly?: boolean | null;
    isStatic?: boolean | null;
    source?: Script | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomProperty.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 1 /* CustomPropertyType.MEMBER */;
    }
    if (_type === null) {
      throw new Error(`CustomProperty.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomProperty.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
    }
    if (_cardinality === null) {
      throw new Error(`CustomProperty.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`CustomProperty.scalarType is required`);
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
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _value = options.value ?? null;
    this.value = _value;
    let _valueFactory = options.valueFactory ?? null;
    this.valueFactory = _valueFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this.nodeConstraint = _nodeConstraint;
    let _edgeType = options.edgeType ?? null;
    this.edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this.cascade = _cascade;
    let _isRequired = options.isRequired ?? null;
    this.isRequired = _isRequired;
    let _isUnique = options.isUnique ?? null;
    this.isUnique = _isUnique;
    let _isComputed = options.isComputed ?? null;
    this.isComputed = _isComputed;
    let _isReadonly = options.isReadonly ?? null;
    this.isReadonly = _isReadonly;
    let _isStatic = options.isStatic ?? null;
    this.isStatic = _isStatic;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.metatype == StructType.NODE_REFERENCE
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
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
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (!(this.valueFactory === other.valueFactory)) {
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
    if (!(this.edgeType === other.edgeType)) {
      return false;
    }
    if (!(this.cascade === other.cascade)) {
      return false;
    }
    if (!(this.isRequired === other.isRequired)) {
      return false;
    }
    if (!(this.isUnique === other.isUnique)) {
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
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
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
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.type) & 0xffffffff;
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
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    if (this.keyType !== null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.valueFactory !== null) {
      h = (h * 31 + this.valueFactory) & 0xffffffff;
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
    if (this.edgeType !== null) {
      h = (h * 31 + this.edgeType) & 0xffffffff;
    }
    if (this.cascade !== null) {
      h = (h * 31 + this.cascade) & 0xffffffff;
    }
    if (this.isRequired !== null) {
      h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    }
    if (this.isUnique !== null) {
      h = (h * 31 + hashBool(this.isUnique)) & 0xffffffff;
    }
    if (this.isComputed !== null) {
      h = (h * 31 + hashBool(this.isComputed)) & 0xffffffff;
    }
    if (this.isReadonly !== null) {
      h = (h * 31 + hashBool(this.isReadonly)) & 0xffffffff;
    }
    if (this.isStatic !== null) {
      h = (h * 31 + hashBool(this.isStatic)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      nodeType: NodeType.CUSTOM_PROPERTY,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
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
    if (this.definition !== null) {
      propertyReprs.push(`definition=${this.definition?.repr()}`);
    }
    if (this.keyType !== null) {
      propertyReprs.push(`keyType=${this.keyType.repr()}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<CustomProperty '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomProperty.__packValue__(this);
  }

  static __packValue__(object: CustomProperty): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 320;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["24"] = object.orderKey;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
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
      objectValue["45"] = object.structType;
    }
    if (object.definitionPtr != null) {
      objectValue["46"] = object.definitionPtr.toValue();
    }
    if (object.keyType != null) {
      objectValue["48"] = object.keyType.toValue();
    }
    if (object.value != null) {
      objectValue["50"] = object.value.toValue();
    }
    if (object.valueFactory != null) {
      objectValue["51"] = object.valueFactory;
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
    if (object.edgeType != null) {
      objectValue["70"] = object.edgeType;
    }
    if (object.cascade != null) {
      objectValue["71"] = object.cascade;
    }
    if (object.isRequired != null) {
      objectValue["80"] = object.isRequired;
    }
    if (object.isUnique != null) {
      objectValue["81"] = object.isUnique;
    }
    if (object.isComputed != null) {
      objectValue["82"] = object.isComputed;
    }
    if (object.isReadonly != null) {
      objectValue["83"] = object.isReadonly;
    }
    if (object.isStatic != null) {
      objectValue["84"] = object.isStatic;
    }
    if (object.sourcePtr != null) {
      objectValue["210"] = object.sourcePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const primitiveTypeValue = objectValue["42"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["43"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["44"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["45"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const definitionPtrValue = objectValue["46"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyTypeValue = objectValue["48"];
    const unpackedKeyType =
      keyTypeValue != undefined
        ? _Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectValue["50"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueFactoryValue = objectValue["51"];
    const unpackedValueFactory = valueFactoryValue != undefined ? Number(valueFactoryValue) : null;
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
    const edgeTypeValue = objectValue["70"];
    const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectValue["71"];
    const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : null;
    const isRequiredValue = objectValue["80"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const isUniqueValue = objectValue["81"];
    const unpackedIsUnique = isUniqueValue != undefined ? isUniqueValue : null;
    const isComputedValue = objectValue["82"];
    const unpackedIsComputed = isComputedValue != undefined ? isComputedValue : null;
    const isReadonlyValue = objectValue["83"];
    const unpackedIsReadonly = isReadonlyValue != undefined ? isReadonlyValue : null;
    const isStaticValue = objectValue["84"];
    const unpackedIsStatic = isStaticValue != undefined ? isStaticValue : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const sourcePtrValue = objectValue["210"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CustomProperty({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      cardinality: Number(objectValue["40"]),
      scalarType: Number(objectValue["41"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      structType: unpackedStructType,
      definition: unpackedDefinitionPtr,
      keyType: unpackedKeyType,
      value: unpackedValue,
      valueFactory: unpackedValueFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      isRequired: unpackedIsRequired,
      isUnique: unpackedIsUnique,
      isComputed: unpackedIsComputed,
      isReadonly: unpackedIsReadonly,
      isStatic: unpackedIsStatic,
      space: unpackedSpacePtr,
      name: objectValue["31"],
      icon: unpackedIcon,
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      orderKey: objectValue["24"],
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    return CustomProperty.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CustomPropertyProto {
    return CustomProperty.__packProto__(this);
  }

  static __packProto__(object: CustomProperty): CustomPropertyProto {
    const objectProto: Partial<CustomPropertyProto> = { metatype: 320 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.orderKey = object.orderKey;
    objectProto.type = Number(object.type) as CustomPropertyTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
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
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.keyType != null) {
      objectProto.keyType = object.keyType.toProto();
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    if (object.valueFactory != null) {
      objectProto.valueFactory = Number(object.valueFactory) as ValueFactoryProto;
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
    if (object.edgeType != null) {
      objectProto.edgeType = Number(object.edgeType) as EdgeTypeProto;
    }
    if (object.cascade != null) {
      objectProto.cascade = Number(object.cascade) as CascadeActionProto;
    }
    if (object.isRequired != null) {
      objectProto.isRequired = object.isRequired;
    }
    if (object.isUnique != null) {
      objectProto.isUnique = object.isUnique;
    }
    if (object.isComputed != null) {
      objectProto.isComputed = object.isComputed;
    }
    if (object.isReadonly != null) {
      objectProto.isReadonly = object.isReadonly;
    }
    if (object.isStatic != null) {
      objectProto.isStatic = object.isStatic;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    return objectProto as CustomPropertyProto;
  }

  static __unpackProto__(
    objectProto: CustomPropertyProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
    return new CustomProperty({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      type: Number(objectProto.type) as CustomPropertyType,
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
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      keyType:
        objectProto.keyType != undefined
          ? _Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      value:
        objectProto.value != undefined
          ? _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
      valueFactory:
        objectProto.valueFactory != undefined
          ? (Number(objectProto.valueFactory) as ValueFactory)
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
      edgeType:
        objectProto.edgeType != undefined ? (Number(objectProto.edgeType) as EdgeType) : null,
      cascade:
        objectProto.cascade != undefined ? (Number(objectProto.cascade) as CascadeAction) : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      isUnique: objectProto.isUnique != undefined ? objectProto.isUnique : null,
      isComputed: objectProto.isComputed != undefined ? objectProto.isComputed : null,
      isReadonly: objectProto.isReadonly != undefined ? objectProto.isReadonly : null,
      isStatic: objectProto.isStatic != undefined ? objectProto.isStatic : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      orderKey: objectProto.orderKey,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CustomPropertyProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    return CustomProperty.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): CustomProperty {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomPropertyProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

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

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_PROPERTY, CustomProperty);
/* ==== DESTACK_GENERATED_END:NODE:320 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:506 ==== */
/**
 * CustomPropertyType
 */
export enum CustomPropertyType {
  MEMBER = 1,
  INPUT = 2,
  OUTPUT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CUSTOM_PROPERTY_TYPE, CustomPropertyType);
/* ==== DESTACK_GENERATED_END:ENUM:506 ==== */
