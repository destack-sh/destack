import { PlaneShape, PrimitiveType, IsTaggable, GaugeMetric, EffectStyle, CollectionConstraint, FontStyle, Machine, Interruption, HistogramMetric, Log, Tag, IsTracked, IsSourceable, Entity, Database, FriendshipInvite, NodeReference, Entitlement, Layer, Branch, BorderStyle, CounterMetric, Action, EditEvent, AnnotationShape, EdgeType, Session, FriendshipInviteEvent, NumberInputView, CustomEvent, Trigger, ShadowStyle, Tagging, User, CustomEntityDefinition, SliderInputView, Snapshot, TransitionStyle, CustomViewDefinition, Struct, NumberConstraint, Handle, Friendship, ThreadCursor, Invite, ColorStyle, Role, DefaultFactory, Message, QueryConnection, ScreenCursor, Link, Value, CustomView, Span, WizardView, TypeCardinality, NodeType, TriggerEvent, CustomStructDefinition, TimerEvent, CustomEntity, Organization, SanctionEvent, Thread, LineShape, GaugeMeasurement, Palette, RunEvent, BuiltinObject, InviteEvent, Team, IsDeletable, Membership, Permission, Spatial, StringConstraint, Option, EnumType, Timer, ScalarType, CascadeAction, HistogramMeasurement, CustomEnumDefinition, Service, Node, Scene, CustomEventDefinition, Notification, SceneEvent, Folder, NodeConstraint, MembershipEvent, GradientStyle, Run, Window, Canvas, Variant, LabelView, Graph, ThreadView, MaterializationType, StructFrozen, CounterMeasurement, Sanction, Script, Environment, Star, Reaction, SplitView, RoleEvent, Type, FrameView, Theme, EntitlementEvent, NotificationEvent, Follow, File, IsOrdered, Client, Agent, Space, ArrowShape, TextView, StructType, Icon, Route, Supergraph, EventCursor, FillStyle } from '@/language';
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
  default: Value | null;
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
    default: Value | null,
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
    this.default = default;
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
    default?: Value | null,
    defaultFactory?: DefaultFactory | null,
    collectionConstraint?: CollectionConstraint | null,
    stringConstraint?: StringConstraint | null,
    numberConstraint?: NumberConstraint | null,
    nodeConstraint?: NodeConstraint | null,
    edgeType?: EdgeType | null,
    cascade?: CascadeAction | null
  }): Field {

    return new Field(

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