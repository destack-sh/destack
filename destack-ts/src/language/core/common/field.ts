import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  CascadeAction,
  CollectionConstraint,
  CustomEntityDefinition,
  DefaultFactory,
  EdgeType,
  Entity,
  EnumType,
  Graph,
  HasIcon,
  HasName,
  Icon,
  IsDeletable,
  IsExtensible,
  IsSourceable,
  IsSubject,
  IsTaggable,
  MaterializationType,
  Node,
  NodeConstraint,
  NodeReference,
  NodeType,
  NumberConstraint,
  PrimitiveType,
  QueryConnection,
  ScalarType,
  Session,
  Spatial,
  StringConstraint,
  StructType,
  Supergraph,
  TraitType,
  Type,
  TypeCardinality,
  Value,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  CascadeActionProto,
  DefaultFactoryProto,
  EdgeTypeProto,
  EnumTypeProto,
  FieldProto,
  FieldTypeProto,
  MaterializationTypeProto,
  NodeTypeProto,
  PrimitiveTypeProto,
  ScalarTypeProto,
  StructTypeProto,
  TypeCardinalityProto,
} from "@destack/proto";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2580 ==== */
/**
 * FieldType
 */
export enum FieldType {
  MEMBER = 1,
  INPUT = 2,
  OUTPUT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FIELD_TYPE, FieldType);
/* ==== DESTACK_GENERATED_END:ENUM:2580 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2520 ==== */
/**
 * A Field is a custom attribute of a CustomStructDefinition or an IsExtensible.
 */
export class Field extends Node implements Spatial, Entity, HasName, HasIcon, IsTaggable, IsDeletable, IsSourceable {
  static metatype: NodeType = NodeType.FIELD;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.ORDERED,
    TraitType.SOURCEABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.PLANE_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.RUN,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SCENE,
    NodeType.INTERRUPTION,
    NodeType.SCRIPT,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.SERVICE,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ACTION,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.FIELD,
    NodeType.CANVAS,
  ];
  static __childTypes__: NodeType[] = [NodeType.FIELD, NodeType.OPTION, NodeType.TAGGING];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.RUN,
    NodeType.FRAME_VIEW,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.SCENE,
    NodeType.INTERRUPTION,
    NodeType.SPLIT_VIEW,
    NodeType.SCRIPT,
    NodeType.LAYER,
    NodeType.SERVICE,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ACTION,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.FIELD,
    NodeType.AGENT,
    NodeType.TEXT_VIEW,
    NodeType.FOLDER,
    NodeType.THREAD_VIEW,
    NodeType.CANVAS,
  ];
  static __descendantTypes__: NodeType[] = [NodeType.FIELD, NodeType.OPTION, NodeType.TAGGING];

  /**
   * Field.parent
   */
  get parent(): (Node & IsExtensible) | Field | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsExtensible) | Field | null;
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
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
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
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
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
   * Field.type
   */
  type: FieldType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /**
   * Field.cardinality
   */
  cardinality: TypeCardinality;

  /**
   * Field.scalarType
   */
  scalarType: ScalarType;

  /**
   * Field.primitiveType
   */
  primitiveType: PrimitiveType | null;

  /**
   * Field.enumType
   */
  enumType: EnumType | null;

  /**
   * Field.nodeType
   */
  nodeType: NodeType | null;

  /**
   * Field.nodeDefinition
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
   * Field.structType
   */
  structType: StructType | null;

  /**
   * Field.baseType
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
   * Field.keyType
   */
  keyType: Type | null;

  /**
   * Field.isRequired
   */
  isRequired: boolean | null;

  /**
   * Field.defaultValue
   */
  defaultValue: Value | null;

  /**
   * Field.defaultFactory
   */
  defaultFactory: DefaultFactory | null;

  /**
   * Field.collectionConstraint
   */
  collectionConstraint: CollectionConstraint | null;

  /**
   * Field.stringConstraint
   */
  stringConstraint: StringConstraint | null;

  /**
   * Field.numberConstraint
   */
  numberConstraint: NumberConstraint | null;

  /**
   * Field.nodeConstraint
   */
  nodeConstraint: NodeConstraint | null;

  /**
   * Field.edgeType
   */
  edgeType: EdgeType | null;

  /**
   * Field.cascade
   */
  cascade: CascadeAction | null;

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
    parent?: (Node & IsExtensible) | Field | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type?: FieldType;
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
    defaultValue?: Value | null;
    defaultFactory?: DefaultFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Field.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Field.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = FieldType.MEMBER;
    }
    if (_type === null) {
      throw new Error(`Field.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Field.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = TypeCardinality.SCALAR;
    }
    if (_cardinality === null) {
      throw new Error(`Field.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`Field.scalarType is required`);
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
    let _source = options.source ?? null;
    if (_source != null && _source instanceof Node) {
      _source = _source.toRef();
    }
    this.sourcePtr = _source;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    if (
      (this.primitiveType == null) !== (other.primitiveType == null) ||
      (this.primitiveType != null && !(this.primitiveType === other.primitiveType))
    ) {
      return false;
    }
    if (
      (this.enumType == null) !== (other.enumType == null) ||
      (this.enumType != null && !(this.enumType === other.enumType))
    ) {
      return false;
    }
    if (
      (this.nodeType == null) !== (other.nodeType == null) ||
      (this.nodeType != null && !(this.nodeType === other.nodeType))
    ) {
      return false;
    }
    if (
      (this.structType == null) !== (other.structType == null) ||
      (this.structType != null && !(this.structType === other.structType))
    ) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (
      (this.isRequired == null) !== (other.isRequired == null) ||
      (this.isRequired != null && !(this.isRequired === other.isRequired))
    ) {
      return false;
    }
    if (
      (this.defaultValue == null) !== (other.defaultValue == null) ||
      (this.defaultValue != null && !this.defaultValue.equals(other.defaultValue))
    ) {
      return false;
    }
    if (
      (this.defaultFactory == null) !== (other.defaultFactory == null) ||
      (this.defaultFactory != null && !(this.defaultFactory === other.defaultFactory))
    ) {
      return false;
    }
    if (
      (this.collectionConstraint == null) !== (other.collectionConstraint == null) ||
      (this.collectionConstraint != null && !this.collectionConstraint.equals(other.collectionConstraint))
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
    if (
      (this.edgeType == null) !== (other.edgeType == null) ||
      (this.edgeType != null && !(this.edgeType === other.edgeType))
    ) {
      return false;
    }
    if (
      (this.cascade == null) !== (other.cascade == null) ||
      (this.cascade != null && !(this.cascade === other.cascade))
    ) {
      return false;
    }
    if (!(this.materialization === other.materialization)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if ((this.icon == null) !== (other.icon == null) || (this.icon != null && !this.icon.equals(other.icon))) {
      return false;
    }
    if (
      (this.nodeDefinitionPtr == null) !== (other.nodeDefinitionPtr == null) ||
      (this.nodeDefinitionPtr != null && !(this.nodeDefinitionPtr.id === other.nodeDefinitionPtr.id))
    ) {
      return false;
    }
    if (
      (this.baseTypePtr == null) !== (other.baseTypePtr == null) ||
      (this.baseTypePtr != null && !(this.baseTypePtr.id === other.baseTypePtr.id))
    ) {
      return false;
    }
    if (
      (this.spacePtr == null) !== (other.spacePtr == null) ||
      (this.spacePtr != null && !(this.spacePtr.id === other.spacePtr.id))
    ) {
      return false;
    }
    if (
      (this.sourcePtr == null) !== (other.sourcePtr == null) ||
      (this.sourcePtr != null && !(this.sourcePtr.id === other.sourcePtr.id))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.FIELD,
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

  toValue(): { [key: string]: any } {
    return Field.__packValue__(this);
  }

  static __packValue__(object: Field): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2520;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString();
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
  ): Field {
    const primitiveTypeValue = objectValue["42"];
    const unpackedPrimitiveType = primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["43"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["44"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["46"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const keyTypeValue = objectValue["48"];
    const unpackedKeyType =
      keyTypeValue != undefined ? Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection) : null;
    const isRequiredValue = objectValue["50"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const defaultValueValue = objectValue["55"];
    const unpackedDefaultValue =
      defaultValueValue != undefined
        ? Value.fromValue(defaultValueValue, _session, _supergraph, _graph, _connection)
        : null;
    const defaultFactoryValue = objectValue["56"];
    const unpackedDefaultFactory = defaultFactoryValue != undefined ? Number(defaultFactoryValue) : null;
    const collectionConstraintValue = objectValue["60"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? CollectionConstraint.fromValue(collectionConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const stringConstraintValue = objectValue["61"];
    const unpackedStringConstraint =
      stringConstraintValue != undefined
        ? StringConstraint.fromValue(stringConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const numberConstraintValue = objectValue["62"];
    const unpackedNumberConstraint =
      numberConstraintValue != undefined
        ? NumberConstraint.fromValue(numberConstraintValue, _session, _supergraph, _graph, _connection)
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
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue != undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodeDefinitionValue = objectValue["45"];
    const unpackedNodeDefinition =
      nodeDefinitionValue != undefined
        ? NodeReference.fromValue(nodeDefinitionValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["47"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? NodeReference.fromValue(baseTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue != undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue != undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue != undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourceValue = objectValue["210"];
    const unpackedSource =
      sourceValue != undefined
        ? NodeReference.fromValue(sourceValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Field({
      type: Number(objectValue["30"]),
      cardinality: Number(objectValue["40"]),
      scalarType: Number(objectValue["41"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      structType: unpackedStructType,
      keyType: unpackedKeyType,
      isRequired: unpackedIsRequired,
      defaultValue: unpackedDefaultValue,
      defaultFactory: unpackedDefaultFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      icon: unpackedIcon,
      deletedAt: unpackedDeletedAt,
      orderKey: objectValue["22"],
      parent: unpackedParent,
      nodeDefinition: unpackedNodeDefinition,
      baseType: unpackedBaseType,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      source: unpackedSource,
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
  ): Field {
    return Field.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FieldProto {
    return Field.__packProto__(this);
  }

  static __packProto__(object: Field): FieldProto {
    const objectProto: Partial<FieldProto> = { metatype: 2520 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationTypeProto;
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
    objectProto.type = Number(object.type) as FieldTypeProto;
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
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    return objectProto as FieldProto;
  }

  static __unpackProto__(
    objectProto: FieldProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Field {
    return new Field({
      type: Number(objectProto.type) as FieldType,
      cardinality: Number(objectProto.cardinality) as TypeCardinality,
      scalarType: Number(objectProto.scalarType) as ScalarType,
      primitiveType:
        objectProto.primitiveType != undefined ? (Number(objectProto.primitiveType) as PrimitiveType) : null,
      enumType: objectProto.enumType != undefined ? (Number(objectProto.enumType) as EnumType) : null,
      nodeType: objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      structType: objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      keyType:
        objectProto.keyType != undefined
          ? Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      defaultValue:
        objectProto.defaultValue != undefined
          ? Value.fromProto(objectProto.defaultValue!, _session, _supergraph, _graph, _connection)
          : null,
      defaultFactory:
        objectProto.defaultFactory != undefined ? (Number(objectProto.defaultFactory) as DefaultFactory) : null,
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
          ? StringConstraint.fromProto(objectProto.stringConstraint!, _session, _supergraph, _graph, _connection)
          : null,
      numberConstraint:
        objectProto.numberConstraint != undefined
          ? NumberConstraint.fromProto(objectProto.numberConstraint!, _session, _supergraph, _graph, _connection)
          : null,
      nodeConstraint:
        objectProto.nodeConstraint != undefined
          ? NodeConstraint.fromProto(objectProto.nodeConstraint!, _session, _supergraph, _graph, _connection)
          : null,
      edgeType: objectProto.edgeType != undefined ? (Number(objectProto.edgeType) as EdgeType) : null,
      cascade: objectProto.cascade != undefined ? (Number(objectProto.cascade) as CascadeAction) : null,
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      orderKey: objectProto.orderKey,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      nodeDefinition:
        objectProto.nodeDefinitionPtr != undefined
          ? NodeReference.fromProto(objectProto.nodeDefinitionPtr!, _session, _supergraph, _graph, _connection)
          : null,
      baseType:
        objectProto.baseTypePtr != undefined
          ? NodeReference.fromProto(objectProto.baseTypePtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      source:
        objectProto.sourcePtr != undefined
          ? NodeReference.fromProto(objectProto.sourcePtr!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: FieldProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Field {
    return Field.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FIELD, Field);
/* ==== DESTACK_GENERATED_END:NODE:2520 ==== */
