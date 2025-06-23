import {
  Action,
  Agent,
  AnnotationShape,
  Canvas,
  CascadeAction,
  CollectionConstraint,
  CustomEntity,
  CustomEntityDefinition,
  CustomEnumDefinition,
  CustomStructDefinition,
  CustomView,
  CustomViewDefinition,
  DefaultFactory,
  EdgeType,
  Entity,
  EnumType,
  FrameView,
  Graph,
  Icon,
  Interruption,
  IsDeletable,
  IsOrdered,
  IsSourceable,
  IsTaggable,
  IsTracked,
  LabelView,
  Layer,
  MaterializationType,
  Node,
  NodeConstraint,
  NodeReference,
  NodeType,
  NumberConstraint,
  PlaneShape,
  PrimitiveType,
  QueryConnection,
  Run,
  ScalarType,
  Scene,
  Script,
  Service,
  Session,
  Space,
  Spatial,
  SplitView,
  StringConstraint,
  StructType,
  Supergraph,
  Type,
  TypeCardinality,
  User,
  Value,
} from "@/language";
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
  readonly id: string;
  get parent():
    | CustomEntity
    | CustomEnumDefinition
    | CustomStructDefinition
    | CustomViewDefinition
    | CustomView
    | FrameView
    | LabelView
    | SplitView
    | AnnotationShape
    | Canvas
    | PlaneShape
    | Action
    | Script
    | Service
    | Interruption
    | Run
    | Layer
    | Scene
    | Field
    | null
    | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntity
        | CustomEnumDefinition
        | CustomStructDefinition
        | CustomViewDefinition
        | CustomView
        | FrameView
        | LabelView
        | SplitView
        | AnnotationShape
        | Canvas
        | PlaneShape
        | Action
        | Script
        | Service
        | Interruption
        | Run
        | Layer
        | Scene
        | Field
        | null
        | null;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
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
    id: string;
    parent?:
      | CustomEntity
      | CustomEnumDefinition
      | CustomStructDefinition
      | CustomViewDefinition
      | CustomView
      | FrameView
      | LabelView
      | SplitView
      | AnnotationShape
      | Canvas
      | PlaneShape
      | Action
      | Script
      | Service
      | Interruption
      | Run
      | Layer
      | Scene
      | Field
      | NodeReference
      | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
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
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
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
    this.deletedAt = options.deletedAt ?? null;
    this.orderKey = options.orderKey ?? "a0";
    this.type = options.type ?? FieldType.MEMBER;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.cardinality = options.cardinality ?? TypeCardinality.SCALAR;
    this.scalarType = options.scalarType;
    this.primitiveType = options.primitiveType ?? null;
    this.enumType = options.enumType ?? null;
    this.nodeType = options.nodeType ?? null;
    this.nodeDefinitionPtr =
      options.nodeDefinition != null
        ? options.nodeDefinition.metatype == StructType.NODE_REFERENCE
          ? (options.nodeDefinition as NodeReference)
          : (options.nodeDefinition as Node).toRef()
        : null;
    this.structType = options.structType ?? null;
    this.baseTypePtr =
      options.baseType != null
        ? options.baseType.metatype == StructType.NODE_REFERENCE
          ? (options.baseType as NodeReference)
          : (options.baseType as Node).toRef()
        : null;
    this.keyType = options.keyType ?? null;
    this.isRequired = options.isRequired ?? null;
    this.defaultValue = options.defaultValue ?? null;
    this.defaultFactory = options.defaultFactory ?? null;
    this.collectionConstraint = options.collectionConstraint ?? null;
    this.stringConstraint = options.stringConstraint ?? null;
    this.numberConstraint = options.numberConstraint ?? null;
    this.nodeConstraint = options.nodeConstraint ?? null;
    this.edgeType = options.edgeType ?? null;
    this.cascade = options.cascade ?? null;
    this.sourcePtr =
      options.source != null
        ? options.source.metatype == StructType.NODE_REFERENCE
          ? (options.source as NodeReference)
          : (options.source as Node).toRef()
        : null;
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
