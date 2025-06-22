import { Entitlement, Theme, ThreadView, CustomEvent, ColorStyle, Timer, FrameView, Follow, Machine, Handle, CustomStructDefinition, Environment, Invite, Palette, LabelView, Agent, QueryConnection, RunEvent, EditEvent, IsDeletable, GradientStyle, RoleEvent, Tagging, Session, NotificationEvent, ScreenCursor, BuiltinObject, Role, Tag, SliderInputView, Action, Team, GaugeMetric, FontStyle, ThreadCursor, Text, CounterMeasurement, IsTracked, Organization, CustomEntityDefinition, StructFrozen, Supergraph, InviteEvent, Struct, LineShape, Log, CustomEventDefinition, CustomView, IsReactable, FillStyle, Option, Canvas, IsOwnable, Window, WizardView, User, TransitionStyle, Spatial, SplitView, Snapshot, AnnotationShape, Layer, NumberInputView, Interruption, Permission, File, Script, TextView, Reaction, Variant, Branch, NodeType, Entity, Scene, Link, IsTaggable, Thread, StructType, CustomEntity, Graph, Friendship, SceneEvent, EntitlementEvent, CustomEnumDefinition, Route, Notification, EnumType, Client, Service, Sanction, Span, SanctionEvent, MembershipEvent, TriggerEvent, MaterializationType, TimerEvent, ShadowStyle, Folder, Space, PlaneShape, HistogramMetric, Membership, Node, ArrowShape, CounterMetric, EffectStyle, Run, GaugeMeasurement, BorderStyle, Star, Database, NodeReference, FriendshipInviteEvent, EventCursor, CustomViewDefinition, FriendshipInvite, HistogramMeasurement, Field, Trigger } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:5510 ==== */
export class Message extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOwnable, IsTaggable, IsReactable {
  readonly id: string;
  get parent(): Thread | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Thread | null | null;
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
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
      }
      return null;
  }

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  get thread(): Thread | null | null {
      const nodePtr: NodeReference | null = this.threadPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Thread | null | null;
      }
      return null;
  }

  set thread(value: Thread | null) {
      if (value === null) {
          this.threadPtr = null;
      } else {
          this.threadPtr = value.toRef();
      }
  }
  ;
  threadPtr: NodeReference | null
  editedAt: Temporal.ZonedDateTime | null;
  get replyTo(): Message | null | null {
      const nodePtr: NodeReference | null = this.replyToPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Message | null | null;
      }
      return null;
  }

  set replyTo(value: Message | null) {
      if (value === null) {
          this.replyToPtr = null;
      } else {
          this.replyToPtr = value.toRef();
      }
  }
  ;
  replyToPtr: NodeReference | null
  get forwardedFrom(): Message | null | null {
      const nodePtr: NodeReference | null = this.forwardedFromPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Message | null | null;
      }
      return null;
  }

  set forwardedFrom(value: Message | null) {
      if (value === null) {
          this.forwardedFromPtr = null;
      } else {
          this.forwardedFromPtr = value.toRef();
      }
  }
  ;
  forwardedFromPtr: NodeReference | null
  text: Text | null;
  get node(): Node | null | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null | null;
      }
      return null;
  }

  set node(value: Node | null) {
      if (value === null) {
          this.nodePtr = null;
      } else {
          this.nodePtr = value.toRef();
      }
  }
  ;
  nodePtr: NodeReference | null

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
    ownedByPtr: NodeReference | null,
    threadPtr: NodeReference | null,
    editedAt: Temporal.ZonedDateTime | null,
    replyToPtr: NodeReference | null,
    forwardedFromPtr: NodeReference | null,
    text: Text | null,
    nodePtr: NodeReference | null,
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
    this.ownedByPtr = ownedByPtr;
    this.threadPtr = threadPtr;
    this.editedAt = editedAt;
    this.replyToPtr = replyToPtr;
    this.forwardedFromPtr = forwardedFromPtr;
    this.text = text;
    this.nodePtr = nodePtr;
  }


  static create(options: {
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    thread?: Thread | NodeReference | null,
    editedAt?: Temporal.ZonedDateTime | null,
    replyTo?: Message | NodeReference | null,
    forwardedFrom?: Message | NodeReference | null,
    text?: Text | null,
    node?: Node | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Message {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Message(
      options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? options.ownedBy : options.ownedBy.toRef()) : null,
      options.thread != null ? (options.thread.metatype == StructType.NODE_REFERENCE ? options.thread : options.thread.toRef()) : null,
      options.editedAt ?? null,
      options.replyTo != null ? (options.replyTo.metatype == StructType.NODE_REFERENCE ? options.replyTo : options.replyTo.toRef()) : null,
      options.forwardedFrom != null ? (options.forwardedFrom.metatype == StructType.NODE_REFERENCE ? options.forwardedFrom : options.forwardedFrom.toRef()) : null,
      options.text ?? null,
      options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? options.node : options.node.toRef()) : null,
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
    return new NodeReference(NodeType.MESSAGE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Message[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:5510 ==== */