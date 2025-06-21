import { Layer, Supergraph, Invite, Follow, SanctionEvent, CustomEventDefinition, Theme, TriggerEvent, TextView, Environment, FontStyle, ScalarType, EventCursor, InviteEvent, CounterMetric, Team, Message, IsTracked, Value, TimerEvent, FrameView, EntitlementEvent, Canvas, CascadeAction, AnnotationShape, Run, Trigger, Log, CustomEntity, Space, ColorStyle, Scene, HistogramMetric, Action, Link, CustomViewDefinition, Interruption, Reaction, GradientStyle, ArrowShape, RunEvent, GaugeMeasurement, CustomEvent, Session, EditEvent, Script, CustomStructDefinition, Node, NodeConstraint, Type, Membership, CustomEntityDefinition, Handle, Friendship, CollectionConstraint, GaugeMetric, FriendshipInviteEvent, ThreadView, Database, Agent, Role, Permission, StructFrozen, IsDeletable, FillStyle, BorderStyle, CustomEnumDefinition, SplitView, User, Organization, MaterializationType, BuiltinObject, Spatial, Folder, Graph, Machine, Service, EffectStyle, StringConstraint, NodeType, Span, Tagging, QueryConnection, ShadowStyle, EdgeType, Variant, ThreadCursor, FriendshipInvite, IsTaggable, Route, Window, NotificationEvent, Entity, MembershipEvent, Option, HistogramMeasurement, DefaultFactory, IsSourceable, PlaneShape, CounterMeasurement, Notification, File, EnumType, Entitlement, Struct, NodeReference, NumberConstraint, RoleEvent, ScreenCursor, WizardView, Sanction, CustomView, SliderInputView, LineShape, Thread, Palette, IsOrdered, NumberInputView, StructType, Tag, LabelView, Client, Branch, Icon, Timer, TransitionStyle, SceneEvent, TypeCardinality, Star, PrimitiveType, Snapshot } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:2580 ==== */
export enum FieldType {
  MEMBER = 1,
  INPUT = 2,
  OUTPUT = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:2580 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2520 ==== */
export class Field extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsSourceable {
  readonly id: string;
  get parent(): CustomEntity | CustomEnumDefinition | CustomStructDefinition | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Action | Script | Service | Interruption | Run | Layer | Scene | Field | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntity | CustomEnumDefinition | CustomStructDefinition | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Action | Script | Service | Interruption | Run | Layer | Scene | Field | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
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

  set nodeDefinition(value: CustomEntityDefinition | null) {
      if (value === null) {
          this.nodeDefinitionPtr = null;
      } else {
          this.nodeDefinitionPtr = value.toRef();
      }
  }
  ;
  nodeDefinitionPtr: NodeReference | null
  structType: StructType | null;
  get baseType(): Node | null | null {
      const nodePtr: NodeReference | null = this.baseTypePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null | null;
      }
      return null;
  }

  set baseType(value: Node | null) {
      if (value === null) {
          this.baseTypePtr = null;
      } else {
          this.baseTypePtr = value.toRef();
      }
  }
  ;
  baseTypePtr: NodeReference | null
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
  ;
  sourcePtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    orderKey: string,
    type: FieldType,
    name: string,
    icon: Icon | null,
    cardinality: TypeCardinality,
    scalarType: ScalarType,
    primitiveType: PrimitiveType | null,
    enumType: EnumType | null,
    nodeType: NodeType | null,
    nodeDefinitionPtr: NodeReference | null,
    structType: StructType | null,
    baseTypePtr: NodeReference | null,
    keyType: Type | null,
    isRequired: boolean | null,
    defaultValue: Value | null,
    defaultFactory: DefaultFactory | null,
    collectionConstraint: CollectionConstraint | null,
    stringConstraint: StringConstraint | null,
    numberConstraint: NumberConstraint | null,
    nodeConstraint: NodeConstraint | null,
    edgeType: EdgeType | null,
    cascade: CascadeAction | null,
    sourcePtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.orderKey = orderKey;
    this.type = type;
    this.name = name;
    this.icon = icon;
    this.cardinality = cardinality;
    this.scalarType = scalarType;
    this.primitiveType = primitiveType;
    this.enumType = enumType;
    this.nodeType = nodeType;
    this.nodeDefinitionPtr = nodeDefinitionPtr;
    this.structType = structType;
    this.baseTypePtr = baseTypePtr;
    this.keyType = keyType;
    this.isRequired = isRequired;
    this.defaultValue = defaultValue;
    this.defaultFactory = defaultFactory;
    this.collectionConstraint = collectionConstraint;
    this.stringConstraint = stringConstraint;
    this.numberConstraint = numberConstraint;
    this.nodeConstraint = nodeConstraint;
    this.edgeType = edgeType;
    this.cascade = cascade;
    this.sourcePtr = sourcePtr;
  }


  static create(options: {
    type?: FieldType,
    name: string,
    icon?: Icon | null,
    cardinality?: TypeCardinality,
    scalarType: ScalarType,
    primitiveType?: PrimitiveType | null,
    enumType?: EnumType | null,
    nodeType?: NodeType | null,
    nodeDefinition?: CustomEntityDefinition | NodeReference | null,
    structType?: StructType | null,
    baseType?: Node | NodeReference | null,
    keyType?: Type | null,
    isRequired?: boolean | null,
    defaultValue?: Value | null,
    defaultFactory?: DefaultFactory | null,
    collectionConstraint?: CollectionConstraint | null,
    stringConstraint?: StringConstraint | null,
    numberConstraint?: NumberConstraint | null,
    nodeConstraint?: NodeConstraint | null,
    edgeType?: EdgeType | null,
    cascade?: CascadeAction | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Field {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Field(
      options.type ?? FieldType.MEMBER,
      options.name,
      options.icon ?? null,
      options.cardinality ?? TypeCardinality.SCALAR,
      options.scalarType,
      options.primitiveType ?? null,
      options.enumType ?? null,
      options.nodeType ?? null,
      options.nodeDefinition != null ? (options.nodeDefinition.metatype == StructType.NODE_REFERENCE ? options.nodeDefinition : options.nodeDefinition.toRef()) : null,
      options.structType ?? null,
      options.baseType != null ? (options.baseType.metatype == StructType.NODE_REFERENCE ? options.baseType : options.baseType.toRef()) : null,
      options.keyType ?? null,
      options.isRequired ?? null,
      options.defaultValue ?? null,
      options.defaultFactory ?? null,
      options.collectionConstraint ?? null,
      options.stringConstraint ?? null,
      options.numberConstraint ?? null,
      options.nodeConstraint ?? null,
      options.edgeType ?? null,
      options.cascade ?? null,
      session,
      supergraph,
      options._graph,
      options._connection
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

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.FIELD, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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