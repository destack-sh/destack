import {
  CascadeAction,
  CollectionConstraint,
  CustomEntityDefinition,
  DefaultFactory,
  EdgeType,
  Entity,
  EnumType,
  Graph,
  Icon,
  IsDeletable,
  IsExtensible,
  IsOrdered,
  IsSourceable,
  IsSubject,
  IsTaggable,
  IsTracked,
  MaterializationType,
  Node,
  NodeConstraint,
  NodeReference,
  NodeType,
  NumberConstraint,
  PrimitiveType,
  QueryConnection,
  ScalarType,
  Script,
  Session,
  Space,
  Spatial,
  StringConstraint,
  StructType,
  Supergraph,
  TraitType,
  Type,
  TypeCardinality,
  Value,
} from "@destack/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2580 ==== */
export enum FieldType {
  MEMBER = 1,
  INPUT = 2,
  OUTPUT = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:2580 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2520 ==== */
export class Field
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsSourceable
{
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

  get parent(): (Node & IsExtensible) | Field | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsExtensible) | Field | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  readonly orderKey: string;
  type: FieldType;
  name: string;
  icon: Icon | null;
  cardinality: TypeCardinality;
  scalarType: ScalarType;
  primitiveType: PrimitiveType | null;
  enumType: EnumType | null;
  nodeType: NodeType | null;
  get nodeDefinition(): CustomEntityDefinition | null | null {
    const nodePtr: NodeReference | null = this.nodeDefinitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null | null;
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
  structType: StructType | null;
  get baseType(): Node | null | null {
    const nodePtr: NodeReference | null = this.baseTypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null | null;
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
  keyType: Type | null;
  isRequired: boolean | null;
  defaultValue: Value | null;
  defaultFactory: DefaultFactory | null;
  collectionConstraint: CollectionConstraint | null;
  stringConstraint: StringConstraint | null;
  numberConstraint: NumberConstraint | null;
  nodeConstraint: NodeConstraint | null;
  edgeType: EdgeType | null;
  cascade: CascadeAction | null;
  get source(): Script | null | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null | null;
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
    throw new Error("not implemented");
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
}
/* ==== DESTACK_GENERATED_END:NODE:2520 ==== */
