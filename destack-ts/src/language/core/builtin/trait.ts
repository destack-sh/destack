import { FriendshipInvite, HistogramMeasurement, CounterMeasurement, InviteEvent, Organization, Environment, BorderStyle, Invite, Field, FillStyle, BuiltinObject, Branch, Agent, FontStyle, PlaneShape, AnnotationShape, Scene, SliderInputView, Role, Icon, Trigger, Theme, NotificationEvent, Route, ThreadView, SplitView, RoleEvent, Script, ScreenCursor, Action, Session, ShadowStyle, CustomView, Layer, User, Handle, Timer, Star, MaterializationType, CustomEntityDefinition, Graph, CustomStructDefinition, HistogramMetric, WizardView, FrameView, LabelView, ThreadCursor, Database, Thread, Variant, TextView, RunEvent, GradientStyle, Log, QueryConnection, Node, Client, Reaction, Notification, EntitlementEvent, Friendship, SceneEvent, Canvas, GaugeMeasurement, Value, ArrowShape, Folder, TriggerEvent, Tagging, Tag, CustomEntity, Membership, Span, Follow, Interruption, SanctionEvent, Entitlement, ResourceStatus, Service, Palette, NumberInputView, Team, File, Permission, Link, TransitionStyle, ColorStyle, MembershipEvent, Sanction, CustomViewDefinition, CounterMetric, Snapshot, CustomEnumDefinition, NodeReference, Message, Space, Run, Window, Option, TimerEvent, CustomEventDefinition, LineShape, EffectStyle, Machine, Struct, Supergraph, CustomEvent, GaugeMetric, NodeType, EventCursor, EditEvent, FriendshipInviteEvent } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:50101 ==== */
export enum JoinablePermission {
  INVITE = 1,
  REMOVE = 2,
  KICK = 3,
  BAN = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:50101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:100 ==== */
export interface HasName {
  readonly id: string;
  name: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:101 ==== */
export interface HasSlug {
  readonly id: string;
  slug: string | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:102 ==== */
export interface HasIcon {
  readonly id: string;
  icon: Icon | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:51 ==== */
export interface IsTracked {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
export interface IsVisual {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:9000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50 ==== */
export interface IsFrozen {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:50 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:52 ==== */
export interface IsArchivable {
  readonly id: string;
  readonly archivedAt: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:52 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:53 ==== */
export interface IsDeletable {
  readonly id: string;
  readonly deletedAt: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:53 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:23 ==== */
export interface IsCustomNodeDefinition {
  readonly id: string;
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null
  set prototype(value: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null);
  prototypePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
export interface IsCustomNode {
  readonly id: string;
  get definition(): CustomEntityDefinition | GaugeMetric | CounterMetric | HistogramMetric | CustomViewDefinition | null;
  definitionPtr: NodeReference
}
/* ==== DESTACK_GENERATED_END:TRAIT:24 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:55 ==== */
export interface IsExtensible {
  readonly id: string;
  value: Map<string, Value>;
}
/* ==== DESTACK_GENERATED_END:TRAIT:55 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:56 ==== */
export interface IsOrdered {
  readonly id: string;
  readonly orderKey: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:56 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5532 ==== */
export interface IsReactable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5532 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5530 ==== */
export interface IsStarable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5534 ==== */
export interface IsFollowable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5534 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3003 ==== */
export interface IsSourceable {
  readonly id: string;
  readonly orderKey: string;
  get source(): Script | null
  set source(value: Script | null);
  sourcePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
export interface IsScriptable {
  readonly id: string;
  get script(): Script | null
  set script(value: Script | null);
  scriptPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:3002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3001 ==== */
export interface IsRunnable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3000 ==== */
export interface IsActionable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:500 ==== */
export interface IsOwnable {
  readonly id: string;
  get ownedBy(): Role | Agent | Organization | Team | User | null
  set ownedBy(value: Role | Agent | Organization | Team | User | null);
  ownedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5000 ==== */
export interface IsSettings {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:502 ==== */
export interface IsJoinable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:502 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:505 ==== */
export interface IsSubject {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:505 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:506 ==== */
export interface IsOwner {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:506 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1000 ==== */
export interface IsTaggable {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:1000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:510 ==== */
export interface LikeMembership {
  readonly id: string;
  get member(): Agent | User | null
  set member(value: Agent | User | null);
  memberPtr: NodeReference
}
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
export interface LikeInvite {
  readonly id: string;
  get member(): Agent | User | null
  set member(value: Agent | User | null);
  memberPtr: NodeReference
}
/* ==== DESTACK_GENERATED_END:TRAIT:511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1001 ==== */
export interface LikeTag {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:1001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5535 ==== */
export interface LikeFollow {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5535 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
export interface Global {
  readonly id: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
export interface Spatial {
  readonly id: string;
  get space(): Space | null
  set space(value: Space | null);
  spacePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10 ==== */
export interface Entity {
  readonly id: string;
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
export interface Particle {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
export interface Analytic {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
export interface Indexed {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:13 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:21 ==== */
export interface Resource {
  readonly id: string;
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
  status: ResourceStatus;
  targetStatus: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
export interface Metric {
  readonly id: string;
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null
  set prototype(value: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null);
  prototypePtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
  readonly orderKey: string;
  get source(): Script | null
  set source(value: Script | null);
  sourcePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
export interface Measurement {
  readonly id: string;
  get definition(): GaugeMetric | CounterMetric | HistogramMetric | null
  set definition(value: GaugeMetric | CounterMetric | HistogramMetric | null);
  definitionPtr: NodeReference
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:4011 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:22 ==== */
export interface Event {
  readonly id: string;
  get space(): Space | null
  set space(value: Space | null);
  spacePtr: NodeReference | null
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
  get node(): Node | null
  set node(value: Node | null);
  nodePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */