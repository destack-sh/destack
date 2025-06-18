import { FriendshipInvite, HistogramMeasurement, CounterMeasurement, InviteEvent, IsDeletable, Organization, Environment, BorderStyle, Invite, Field, FillStyle, BuiltinObject, IsOwnable, Branch, Agent, FontStyle, IsReactable, PlaneShape, AnnotationShape, Scene, SliderInputView, Role, Trigger, Theme, NotificationEvent, Route, ThreadView, SplitView, RoleEvent, Script, ScreenCursor, Action, Session, Entity, ShadowStyle, CustomView, Layer, User, Handle, Timer, Star, MaterializationType, CustomEntityDefinition, Graph, CustomStructDefinition, HistogramMetric, WizardView, FrameView, LabelView, ThreadCursor, Database, Thread, Variant, TextView, RunEvent, GradientStyle, Log, QueryConnection, Node, Client, Reaction, Notification, EntitlementEvent, Friendship, SceneEvent, Canvas, GaugeMeasurement, ArrowShape, Folder, TriggerEvent, Tagging, IsTracked, Tag, CustomEntity, Membership, Span, IsTaggable, Follow, Interruption, SanctionEvent, Entitlement, Service, Palette, NumberInputView, Team, File, Permission, Link, Spatial, TransitionStyle, ColorStyle, MembershipEvent, Sanction, CustomViewDefinition, CounterMetric, Snapshot, CustomEnumDefinition, NodeReference, EditEvent, Run, Window, Option, TimerEvent, CustomEventDefinition, LineShape, EffectStyle, Machine, Struct, Text, Supergraph, CustomEvent, GaugeMetric, NodeType, EventCursor, Space, FriendshipInviteEvent } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:5510 ==== */
export class Message extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOwnable, IsTaggable, IsReactable {
  readonly id: string;
  get parent(): Thread | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Thread | null;
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
  get ownedBy(): Role | Agent | Organization | Team | User | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null;
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
  get thread(): Thread | null {
      const nodePtr: NodeReference | null = this.threadPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Thread | null;
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
  get replyTo(): Message | null {
      const nodePtr: NodeReference | null = this.replyToPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Message | null;
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
  get forwardedFrom(): Message | null {
      const nodePtr: NodeReference | null = this.forwardedFromPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Message | null;
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
  get node(): Node | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null;
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


  static create(): Message {

    return new Message();
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