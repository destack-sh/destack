import { Folder, Session, Timer, Snapshot, Option, ScreenCursor, ShadowStyle, Interruption, NodeType, Action, Window, EffectStyle, Trigger, EventCursor, SliderInputView, HistogramMetric, CustomView, LabelView, Environment, ThreadView, TransitionStyle, Space, CustomViewDefinition, Script, EnumType, Client, Layer, TextView, ColorStyle, FriendshipInviteEvent, Field, InviteEvent, Agent, Tagging, IsTaggable, Follow, CustomEventDefinition, AnnotationShape, activeSession, StructType, RunEvent, CustomEvent, Role, Branch, HistogramMeasurement, NotificationEvent, WizardView, TriggerEvent, Variant, SceneEvent, EditEvent, IsReactable, Handle, Reaction, Text, RoleEvent, CustomStructDefinition, Star, NumberInputView, Route, GaugeMetric, CustomEntityDefinition, File, ACTIVE_SESSION, LineShape, ThreadCursor, Machine, Tag, SanctionEvent, FrameView, Friendship, Supergraph, Database, Thread, IsOwnable, FontStyle, Link, User, Spatial, Team, MembershipEvent, ArrowShape, BorderStyle, EntitlementEvent, Entity, SplitView, Node, IsTracked, CounterMeasurement, PlaneShape, BuiltinObject, Organization, Scene, StructFrozen, GradientStyle, Entitlement, Graph, Service, Struct, Permission, Sanction, Canvas, Invite, Span, Palette, QueryConnection, GaugeMeasurement, NodeReference, MaterializationType, CustomEnumDefinition, CounterMetric, TimerEvent, Membership, Run, Theme, IsDeletable, CustomEntity, FriendshipInvite, Notification, Log, FillStyle } from '@/language';
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

  set ownedBy(node: Role | Agent | Organization | Team | User | null) {
      if (node === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = node.toRef();
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

  set thread(node: Thread | null) {
      if (node === null) {
          this.threadPtr = null;
      } else {
          this.threadPtr = node.toRef();
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

  set replyTo(node: Message | null) {
      if (node === null) {
          this.replyToPtr = null;
      } else {
          this.replyToPtr = node.toRef();
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

  set forwardedFrom(node: Message | null) {
      if (node === null) {
          this.forwardedFromPtr = null;
      } else {
          this.forwardedFromPtr = node.toRef();
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

  set node(node: Node | null) {
      if (node === null) {
          this.nodePtr = null;
      } else {
          this.nodePtr = node.toRef();
      }
  }
  ;
  nodePtr: NodeReference | null

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
    this.threadPtr = options.thread != null ? (options.thread.metatype == StructType.NODE_REFERENCE ? (options.thread as NodeReference) : (options.thread as Node).toRef()) : null;
    this.editedAt = options.editedAt ?? null;
    this.replyToPtr = options.replyTo != null ? (options.replyTo.metatype == StructType.NODE_REFERENCE ? (options.replyTo as NodeReference) : (options.replyTo as Node).toRef()) : null;
    this.forwardedFromPtr = options.forwardedFrom != null ? (options.forwardedFrom.metatype == StructType.NODE_REFERENCE ? (options.forwardedFrom as NodeReference) : (options.forwardedFrom as Node).toRef()) : null;
    this.text = options.text ?? null;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
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