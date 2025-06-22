import { EditType, Folder, Session, Timer, Snapshot, Option, ScreenCursor, ShadowStyle, Interruption, NodeType, Action, Window, Indexed, EffectStyle, Trigger, EventCursor, SliderInputView, HistogramMetric, Value, CustomView, LabelView, Environment, ThreadView, Message, TransitionStyle, IsSourceable, Space, CustomViewDefinition, Script, EnumType, Client, Layer, TextView, ColorStyle, FriendshipInviteEvent, Field, InviteEvent, Agent, Tagging, IsTaggable, Follow, AnnotationShape, Event, activeSession, StructType, RunEvent, Role, Branch, HistogramMeasurement, NotificationEvent, WizardView, TriggerEvent, Variant, SceneEvent, Reaction, Handle, RoleEvent, CustomStructDefinition, Star, NumberInputView, Route, GaugeMetric, CustomEntityDefinition, File, ACTIVE_SESSION, LineShape, ThreadCursor, IsOrdered, Machine, Tag, SanctionEvent, FrameView, Friendship, Supergraph, EditOperation, Database, Thread, FontStyle, Link, User, Analytic, Spatial, Team, MembershipEvent, ArrowShape, BorderStyle, EntitlementEvent, SplitView, Entity, Node, IsTracked, CounterMeasurement, PlaneShape, BuiltinObject, Organization, Scene, StructFrozen, IsFrozen, GradientStyle, Entitlement, Graph, Service, Struct, Permission, Particle, Sanction, Canvas, Invite, Span, Palette, QueryConnection, GaugeMeasurement, NodeReference, CustomEnumDefinition, MaterializationType, CounterMetric, TimerEvent, Membership, PropertyReference, Run, Theme, CustomEntity, FriendshipInvite, Notification, Log, FillStyle } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:4202 ==== */
export class EditEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked, IsTaggable {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
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
  type: EditType;
  operation: EditOperation | null;
  get node(): Node | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null;
      }
      return null;
  }

  set node(node: Node) {
      this.nodePtr = node.toRef();
  }
  ;
  nodePtr: NodeReference
  propPtr: PropertyReference | null;
  get field(): Field | null | null {
      const nodePtr: NodeReference | null = this.fieldPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Field | null | null;
      }
      return null;
  }

  set field(node: Field | null) {
      if (node === null) {
          this.fieldPtr = null;
      } else {
          this.fieldPtr = node.toRef();
      }
  }
  ;
  fieldPtr: NodeReference | null
  key: Value | null;
  value: Value | null;

  constructor(options: {
    type: EditType,
    operation?: EditOperation | null,
    node: Node | NodeReference,
    propPtr?: PropertyReference | null,
    field?: Field | NodeReference | null,
    key?: Value | null,
    value?: Value | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.operation = options.operation ?? null;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
    this.propPtr = options.propPtr ?? null;
    this.fieldPtr = options.field != null ? (options.field.metatype == StructType.NODE_REFERENCE ? (options.field as NodeReference) : (options.field as Node).toRef()) : null;
    this.key = options.key ?? null;
    this.value = options.value ?? null;
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
    return new NodeReference(NodeType.EDIT_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "EditEvent[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:4202 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4200 ==== */
export class CustomEventDefinition extends Node implements Spatial, Entity, IsTracked, IsOrdered, IsSourceable {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
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
  readonly orderKey: string;
  name: string;
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
    name: string,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.name = options.name;
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
    return new NodeReference(NodeType.CUSTOM_EVENT_DEFINITION, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:4200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4201 ==== */
export class CustomEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
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
  get node(): Node | null | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null | null;
      }
      return null;
  }

  set node(node: Node | null) {
      if (node === null) {
          this.nodePtr = null;
      } else {
          this.nodePtr = node.toRef();
      }
  }
  ;
  nodePtr: NodeReference | null
  get definition(): CustomEventDefinition | null {
      const nodePtr: NodeReference | null = this.definitionPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEventDefinition | null;
      }
      return null;
  }

  set definition(node: CustomEventDefinition) {
      this.definitionPtr = node.toRef();
  }
  ;
  definitionPtr: NodeReference

  constructor(options: {
    node?: Node | NodeReference | null,
    definition: CustomEventDefinition | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
    this.definitionPtr = options.definition != null ? (options.definition.metatype == StructType.NODE_REFERENCE ? (options.definition as NodeReference) : (options.definition as Node).toRef()) : null;
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
    return new NodeReference(NodeType.CUSTOM_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "CustomEvent[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:4201 ==== */