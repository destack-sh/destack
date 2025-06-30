import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  CascadeAction,
  DefaultFactory,
  EdgeType,
  EnumType,
  NodeType,
  PrimitiveType,
  ScalarType,
  StructType,
  TypeCardinality,
} from "@destack/language/core/builtin/common";
import { CustomEntityDefinition, Entity } from "@destack/language/core/builtin/entity";
import { Node } from "@destack/language/core/builtin/node";
import { NodeReference } from "@destack/language/core/builtin/relation";
import {
  HasIcon,
  HasName,
  IsDeletable,
  IsExtensible,
  IsSourceable,
  IsSpatial,
  IsSubject,
  IsTaggable,
} from "@destack/language/core/builtin/trait";
import { Icon } from "@destack/language/core/common/icon";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  StringConstraint,
  Type,
} from "@destack/language/core/common/type";
import { Value } from "@destack/language/core/common/value";
import { QueryConnection } from "@destack/language/core/runtime/connection";
import { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import { Session } from "@destack/language/core/runtime/session";
import { Script } from "@destack/language/logic";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  CascadeActionProto,
  CustomPropertyProto,
  CustomPropertyTypeProto,
  DefaultFactoryProto,
  EdgeTypeProto,
  EnumTypeProto,
  NodeTypeProto,
  PrimitiveTypeProto,
  ScalarTypeProto,
  StructTypeProto,
  TypeCardinalityProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2580 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2580 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2520 ==== */
/**
 * A CustomProperty is a custom attribute of a CustomStructDefinition or an IsExtensible.
 */
export class CustomProperty
  extends Entity
  implements IsSpatial, HasName, HasIcon, IsTaggable, IsDeletable, IsSourceable
{
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY;

  /**
   * CustomProperty.parent
   */
  get parent(): (Node & IsExtensible) | CustomProperty | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsExtensible) | CustomProperty | null;
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
   * IsOrdered.orderKey
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
   * CustomProperty.nodeDefinition
   */
  get nodeDefinition(): CustomEntityDefinition | null {
    const nodePtr: NodeReference | null = this.nodeDefinitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
    }
    return null;
  }
  set nodeDefinition(node: CustomEntityDefinition | null) {
    if (node === null) {
      this.nodeDefinitionPtr = null;
    } else {
      this.nodeDefinitionPtr = node.toRef();
    }
  }
  nodeDefinitionPtr: NodeReference | null;

  /**
   * CustomProperty.structType
   */
  structType: StructType | null;

  /**
   * CustomProperty.baseType
   */
  get baseType(): Node | null {
    const nodePtr: NodeReference | null = this.baseTypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set baseType(node: Node | null) {
    if (node === null) {
      this.baseTypePtr = null;
    } else {
      this.baseTypePtr = node.toRef();
    }
  }
  baseTypePtr: NodeReference | null;

  /**
   * CustomProperty.keyType
   */
  keyType: Type | null;

  /**
   * CustomProperty.isRequired
   */
  isRequired: boolean | null;

  /**
   * CustomProperty.isUnique
   */
  isUnique: boolean | null;

  /**
   * CustomProperty.defaultValue
   */
  defaultValue: Value | null;

  /**
   * CustomProperty.defaultFactory
   */
  defaultFactory: DefaultFactory | null;

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
    parent?: (Node & IsExtensible) | CustomProperty | NodeReference | null;
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
    nodeDefinition?: CustomEntityDefinition | NodeReference | null;
    structType?: StructType | null;
    baseType?: Node | NodeReference | null;
    keyType?: Type | null;
    isRequired?: boolean | null;
    isUnique?: boolean | null;
    defaultValue?: Value | null;
    defaultFactory?: DefaultFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
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
      _type = CustomPropertyType.MEMBER;
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
      _cardinality = TypeCardinality.SCALAR;
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
    let _nodeDefinition = options.nodeDefinition ?? null;
    if (_nodeDefinition != null && _nodeDefinition instanceof Node) {
      _nodeDefinition = _nodeDefinition.toRef();
    }
    this.nodeDefinitionPtr = _nodeDefinition;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _baseType = options.baseType ?? null;
    if (_baseType != null && _baseType instanceof Node) {
      _baseType = _baseType.toRef();
    }
    this.baseTypePtr = _baseType;
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
    let _edgeType = options.edgeType ?? null;
    this.edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this.cascade = _cascade;
    let _isReadonly = options.isReadonly ?? null;
    this.isReadonly = _isReadonly;
    let _isStatic = options.isStatic ?? null;
    this.isStatic = _isStatic;
    let _source = options.source ?? null;
    if (_source != null && _source instanceof Node) {
      _source = _source.toRef();
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
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
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
    if (!(this.nodeDefinitionPtr?.id === other.nodeDefinitionPtr?.id)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (!(this.baseTypePtr?.id === other.baseTypePtr?.id)) {
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
    if (!(this.edgeType === other.edgeType)) {
      return false;
    }
    if (!(this.cascade === other.cascade)) {
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
    if (this.nodeDefinitionPtr !== null) {
      h = (h * 31 + hashString(this.nodeDefinitionPtr.id)) & 0xffffffff;
    }
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.baseTypePtr !== null) {
      h = (h * 31 + hashString(this.baseTypePtr.id)) & 0xffffffff;
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
    if (this.edgeType !== null) {
      h = (h * 31 + this.edgeType) & 0xffffffff;
    }
    if (this.cascade !== null) {
      h = (h * 31 + this.cascade) & 0xffffffff;
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
    return new NodeReference({
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
    if (this.nodeDefinition !== null) {
      propertyReprs.push(`nodeDefinition=${this.nodeDefinition?.repr()}`);
    }
    if (this.structType !== null) {
      propertyReprs.push(`structType=${StructType[this.structType]}`);
    }
    if (this.baseType !== null) {
      propertyReprs.push(`baseType=${this.baseType?.repr()}`);
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
    objectValue["1"] = 2520;
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
    objectValue["22"] = object.orderKey;
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
    if (object.nodeDefinitionPtr != null) {
      objectValue["45"] = object.nodeDefinitionPtr.toValue();
    }
    if (object.structType != null) {
      objectValue["46"] = object.structType;
    }
    if (object.baseTypePtr != null) {
      objectValue["47"] = object.baseTypePtr.toValue();
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
    if (object.edgeType != null) {
      objectValue["70"] = object.edgeType;
    }
    if (object.cascade != null) {
      objectValue["71"] = object.cascade;
    }
    if (object.isReadonly != null) {
      objectValue["80"] = object.isReadonly;
    }
    if (object.isStatic != null) {
      objectValue["81"] = object.isStatic;
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const primitiveTypeValue = objectValue["42"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["43"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["44"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const nodeDefinitionPtrValue = objectValue["45"];
    const unpackedNodeDefinitionPtr =
      nodeDefinitionPtrValue != undefined
        ? NodeReference.fromValue(
            nodeDefinitionPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const structTypeValue = objectValue["46"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const baseTypePtrValue = objectValue["47"];
    const unpackedBaseTypePtr =
      baseTypePtrValue != undefined
        ? NodeReference.fromValue(baseTypePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyTypeValue = objectValue["48"];
    const unpackedKeyType =
      keyTypeValue != undefined
        ? Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const isRequiredValue = objectValue["50"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const isUniqueValue = objectValue["51"];
    const unpackedIsUnique = isUniqueValue != undefined ? isUniqueValue : null;
    const defaultValueValue = objectValue["55"];
    const unpackedDefaultValue =
      defaultValueValue != undefined
        ? Value.fromValue(defaultValueValue, _session, _supergraph, _graph, _connection)
        : null;
    const defaultFactoryValue = objectValue["56"];
    const unpackedDefaultFactory =
      defaultFactoryValue != undefined ? Number(defaultFactoryValue) : null;
    const collectionConstraintValue = objectValue["60"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? CollectionConstraint.fromValue(
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
        ? StringConstraint.fromValue(
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
        ? NumberConstraint.fromValue(
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
        ? NodeConstraint.fromValue(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const edgeTypeValue = objectValue["70"];
    const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectValue["71"];
    const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : null;
    const isReadonlyValue = objectValue["80"];
    const unpackedIsReadonly = isReadonlyValue != undefined ? isReadonlyValue : null;
    const isStaticValue = objectValue["81"];
    const unpackedIsStatic = isStaticValue != undefined ? isStaticValue : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const sourcePtrValue = objectValue["210"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CustomProperty({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      cardinality: Number(objectValue["40"]),
      scalarType: Number(objectValue["41"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      nodeDefinition: unpackedNodeDefinitionPtr,
      structType: unpackedStructType,
      baseType: unpackedBaseTypePtr,
      keyType: unpackedKeyType,
      isRequired: unpackedIsRequired,
      isUnique: unpackedIsUnique,
      defaultValue: unpackedDefaultValue,
      defaultFactory: unpackedDefaultFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
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
      orderKey: objectValue["22"],
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
    const objectProto: Partial<CustomPropertyProto> = { metatype: 2520 };
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
    if (object.nodeDefinitionPtr != null) {
      objectProto.nodeDefinitionPtr = object.nodeDefinitionPtr.toProto();
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    if (object.baseTypePtr != null) {
      objectProto.baseTypePtr = object.baseTypePtr.toProto();
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
    if (object.edgeType != null) {
      objectProto.edgeType = Number(object.edgeType) as EdgeTypeProto;
    }
    if (object.cascade != null) {
      objectProto.cascade = Number(object.cascade) as CascadeActionProto;
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
    return new CustomProperty({
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
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
      nodeDefinition:
        objectProto.nodeDefinitionPtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodeDefinitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      structType:
        objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      baseType:
        objectProto.baseTypePtr != undefined
          ? NodeReference.fromProto(
              objectProto.baseTypePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      keyType:
        objectProto.keyType != undefined
          ? Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      isUnique: objectProto.isUnique != undefined ? objectProto.isUnique : null,
      defaultValue:
        objectProto.defaultValue != undefined
          ? Value.fromProto(objectProto.defaultValue!, _session, _supergraph, _graph, _connection)
          : null,
      defaultFactory:
        objectProto.defaultFactory != undefined
          ? (Number(objectProto.defaultFactory) as DefaultFactory)
          : null,
      collectionConstraint:
        objectProto.collectionConstraint != undefined
          ? CollectionConstraint.fromProto(
              objectProto.collectionConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      stringConstraint:
        objectProto.stringConstraint != undefined
          ? StringConstraint.fromProto(
              objectProto.stringConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      numberConstraint:
        objectProto.numberConstraint != undefined
          ? NumberConstraint.fromProto(
              objectProto.numberConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      nodeConstraint:
        objectProto.nodeConstraint != undefined
          ? NodeConstraint.fromProto(
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
      isReadonly: objectProto.isReadonly != undefined ? objectProto.isReadonly : null,
      isStatic: objectProto.isStatic != undefined ? objectProto.isStatic : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
          ? Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      source:
        objectProto.sourcePtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
/* ==== DESTACK_GENERATED_END:NODE:2520 ==== */
