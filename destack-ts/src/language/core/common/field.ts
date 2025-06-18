import { FriendshipInvite, HistogramMeasurement, CounterMeasurement, IsSourceable, InviteEvent, IsDeletable, Organization, Environment, BorderStyle, EnumType, Invite, Type, FillStyle, BuiltinObject, Branch, Agent, FontStyle, PlaneShape, AnnotationShape, Scene, SliderInputView, Role, Icon, Trigger, Theme, NotificationEvent, Route, ThreadView, SplitView, RoleEvent, Script, ScreenCursor, NumberConstraint, Action, Session, Entity, ShadowStyle, CustomView, Layer, User, Handle, Timer, Star, MaterializationType, CustomStructDefinition, Graph, CustomEntityDefinition, HistogramMetric, WizardView, FrameView, LabelView, ThreadCursor, Database, Thread, Variant, TextView, RunEvent, GradientStyle, Log, CollectionConstraint, TypeCardinality, CascadeAction, QueryConnection, Node, Client, ScalarType, PrimitiveType, Reaction, Notification, EntitlementEvent, Friendship, Canvas, SceneEvent, GaugeMeasurement, Value, ArrowShape, Folder, TriggerEvent, Tagging, IsTracked, Tag, CustomEntity, Membership, Span, IsTaggable, NodeConstraint, Follow, Interruption, SanctionEvent, Entitlement, Service, Palette, NumberInputView, Team, File, StructType, IsOrdered, StringConstraint, Permission, Link, Spatial, TransitionStyle, DefaultFactory, ColorStyle, MembershipEvent, Sanction, CustomViewDefinition, CounterMetric, Snapshot, CustomEnumDefinition, NodeReference, Message, EditEvent, EdgeType, Run, Window, Option, TimerEvent, CustomEventDefinition, LineShape, EffectStyle, Machine, Struct, Supergraph, CustomEvent, GaugeMetric, NodeType, EventCursor, Space, FriendshipInviteEvent } from '@/language';
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
  get parent(): CustomEntity | CustomEnumDefinition | CustomStructDefinition | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Action | Script | Service | Interruption | Run | Layer | Scene | Field | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntity | CustomEnumDefinition | CustomStructDefinition | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Action | Script | Service | Interruption | Run | Layer | Scene | Field | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
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
  get nodeDefinition(): CustomEntityDefinition | null {
      const nodePtr: NodeReference | null = this.nodeDefinitionPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
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
  get baseType(): Node | null {
      const nodePtr: NodeReference | null = this.baseTypePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null;
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
  get source(): Script | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null;
      }
      return null;
  }

  set source(value: Script | null) {
      if (value === null) {
          this.sourcePtr = null;
      } else {
          this.sourcePtr = value.toRef();
      }
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


  static create(): Field {

    return new Field();
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