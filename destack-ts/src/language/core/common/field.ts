import { Folder, Session, Timer, Snapshot, Option, ScreenCursor, ShadowStyle, Interruption, NodeType, Action, Window, EffectStyle, Trigger, EventCursor, Icon, SliderInputView, HistogramMetric, Value, CustomView, LabelView, Environment, ThreadView, Message, TransitionStyle, IsSourceable, Space, CustomViewDefinition, Script, EnumType, Client, Layer, TextView, ColorStyle, FriendshipInviteEvent, InviteEvent, Agent, Tagging, IsTaggable, Follow, CustomEventDefinition, AnnotationShape, activeSession, StructType, RunEvent, CustomEvent, CascadeAction, Role, Branch, HistogramMeasurement, NotificationEvent, EdgeType, WizardView, TriggerEvent, Variant, SceneEvent, EditEvent, Handle, Reaction, RoleEvent, CustomStructDefinition, Star, NumberInputView, Route, StringConstraint, GaugeMetric, CustomEntityDefinition, File, ACTIVE_SESSION, DefaultFactory, LineShape, ThreadCursor, IsOrdered, NumberConstraint, PrimitiveType, Machine, ScalarType, SanctionEvent, FrameView, Friendship, NodeConstraint, Supergraph, Database, Thread, FontStyle, Link, User, Spatial, Team, MembershipEvent, ArrowShape, BorderStyle, EntitlementEvent, Entity, SplitView, Type, TypeCardinality, Node, IsTracked, CounterMeasurement, PlaneShape, BuiltinObject, Organization, Scene, StructFrozen, GradientStyle, Entitlement, Graph, Service, Struct, Permission, Sanction, Log, Canvas, Invite, Span, Palette, QueryConnection, GaugeMeasurement, NodeReference, CustomEnumDefinition, MaterializationType, CounterMetric, TimerEvent, CollectionConstraint, Membership, Run, Theme, IsDeletable, CustomEntity, FriendshipInvite, Notification, Tag, FillStyle } from '@/language';
import { Temporal } from 'temporal-polyfill';

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

  set nodeDefinition(node: CustomEntityDefinition | null) {
      if (node === null) {
          this.nodeDefinitionPtr = null;
      } else {
          this.nodeDefinitionPtr = node.toRef();
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

  set baseType(node: Node | null) {
      if (node === null) {
          this.baseTypePtr = null;
      } else {
          this.baseTypePtr = node.toRef();
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

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type ?? FieldType.MEMBER;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.cardinality = options.cardinality ?? TypeCardinality.SCALAR;
    this.scalarType = options.scalarType;
    this.primitiveType = options.primitiveType ?? null;
    this.enumType = options.enumType ?? null;
    this.nodeType = options.nodeType ?? null;
    this.nodeDefinitionPtr = options.nodeDefinition != null ? (options.nodeDefinition.metatype == StructType.NODE_REFERENCE ? (options.nodeDefinition as NodeReference) : (options.nodeDefinition as Node).toRef()) : null;
    this.structType = options.structType ?? null;
    this.baseTypePtr = options.baseType != null ? (options.baseType.metatype == StructType.NODE_REFERENCE ? (options.baseType as NodeReference) : (options.baseType as Node).toRef()) : null;
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