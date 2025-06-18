import type { InviteEvent, Folder, ThreadCursor, Friendship, CustomStructDefinition, SliderInputView, WizardView, Tagging, Permission, GaugeMetric, Log, Palette, Run, Canvas, Organization, Link, HistogramMetric, Option, SanctionEvent, ColorStyle, FillStyle, Tag, PlaneShape, Database, CustomViewDefinition, ThreadView, NumberInputView, TextView, Layer, Membership, Follow, Role, BorderStyle, NotificationEvent, HistogramMeasurement, Machine, CounterMeasurement, Interruption, CustomView, MaterializationType, AnnotationShape, RoleEvent, TimerEvent, Star, Entitlement, Scene, Environment, TransitionStyle, Timer, Team, TriggerEvent, Snapshot, Branch, FrameView, Space, Client, EntitlementEvent, Message, RunEvent, FriendshipInvite, Service, EffectStyle, GaugeMeasurement, User, EditEvent, Action, File, Invite, Sanction, CustomEvent, Window, EventCursor, CustomEntityDefinition, Theme, Thread, Trigger, ArrowShape, LineShape, Script, Handle, FontStyle, Notification, ResourceStatus, Field, LabelView, Value, Variant, CustomEnumDefinition, Icon, GradientStyle, CounterMetric, SplitView, ScreenCursor, Span, CustomEventDefinition, FriendshipInviteEvent, SceneEvent, CustomEntity, Agent, ShadowStyle, MembershipEvent, Route, Reaction } from '@/language';
import { BuiltinObject, Struct, Node, NodeReference } from '@/language/core';

import type { TransitionStyle, Option, Star, SanctionEvent, ThreadCursor, SplitView, EntitlementEvent, MembershipEvent, Scene, Layer, GradientStyle, HistogramMetric, Script, FontStyle, EventCursor, EditEvent, User, Permission, CustomEntityDefinition, Run, PlaneShape, Folder, Link, ThreadView, Snapshot, Machine, FillStyle, WizardView, LineShape, Message, Membership, Timer, ArrowShape, BorderStyle, Value, Notification, CustomEvent, CounterMetric, AnnotationShape, Environment, Follow, Client, MaterializationType, Handle, LabelView, FriendshipInvite, Database, Space, Organization, ColorStyle, FrameView, Field, Reaction, Interruption, Sanction, GaugeMetric, Variant, ShadowStyle, Team, CustomView, Log, Role, Thread, Route, HistogramMeasurement, Friendship, CustomEnumDefinition, NotificationEvent, SliderInputView, ScreenCursor, Action, Agent, CustomStructDefinition, Trigger, SceneEvent, EffectStyle, Span, CustomEntity, Theme, CounterMeasurement, ResourceStatus, Branch, CustomEventDefinition, Canvas, Tagging, Tag, TriggerEvent, CustomViewDefinition, RunEvent, Invite, Window, TimerEvent, FriendshipInviteEvent, Entitlement, TextView, InviteEvent, GaugeMeasurement, NumberInputView, RoleEvent, Palette, File, Service, Icon } from '@/language';
import { BuiltinObject, Struct, Node, NodeReference } from '@/language/core';

import type { Thread, Span, NumberInputView, PlaneShape, Scene, CustomEvent, Tagging, GaugeMetric, TextView, TriggerEvent, Icon, Snapshot, FillStyle, AnnotationShape, EffectStyle, Environment, Link, HistogramMeasurement, Organization, EditEvent, SliderInputView, Tag, Canvas, Notification, Palette, Field, Agent, RunEvent, Role, Invite, Client, LineShape, User, Theme, Follow, Folder, TransitionStyle, Option, ScreenCursor, ColorStyle, Star, Permission, Timer, MembershipEvent, WizardView, FontStyle, Entitlement, GradientStyle, Message, Team, Log, CustomEnumDefinition, FrameView, CustomView, Sanction, File, GaugeMeasurement, Machine, Branch, Run, CustomViewDefinition, ThreadView, Membership, CustomEventDefinition, ResourceStatus, Trigger, Friendship, TimerEvent, NotificationEvent, CounterMetric, Handle, CustomEntity, SplitView, ShadowStyle, Window, FriendshipInviteEvent, LabelView, Database, Space, Service, SceneEvent, BorderStyle, SanctionEvent, Value, EventCursor, Variant, Layer, Reaction, HistogramMetric, Route, Interruption, CustomStructDefinition, FriendshipInvite, Action, MaterializationType, ThreadCursor, CustomEntityDefinition, InviteEvent, ArrowShape, EntitlementEvent, Script, CounterMeasurement, RoleEvent } from '@/language';

import type { Follow, Script, File, NotificationEvent, Organization, FriendshipInviteEvent, FrameView, Palette, EditEvent, TriggerEvent, PlaneShape, ScreenCursor, Route, WizardView, Trigger, TransitionStyle, SplitView, Sanction, CustomEnumDefinition, Span, Theme, Thread, Interruption, Timer, Reaction, Value, LineShape, CustomViewDefinition, InviteEvent, RoleEvent, BorderStyle, Action, RunEvent, TextView, ColorStyle, FriendshipInvite, Handle, Star, Run, SanctionEvent, CustomView, Icon, CustomEntityDefinition, User, HistogramMeasurement, Folder, Machine, EventCursor, Client, Role, ShadowStyle, CounterMetric, Log, TimerEvent, EntitlementEvent, CustomEvent, ResourceStatus, Permission, Option, MembershipEvent, Agent, Database, Scene, Invite, ThreadView, CustomEntity, Notification, Friendship, Membership, GaugeMetric, Team, Service, Field, NumberInputView, Message, FillStyle, AnnotationShape, Entitlement, ThreadCursor, Tagging, Layer, FontStyle, GradientStyle, Snapshot, CustomEventDefinition, Link, HistogramMetric, LabelView, SceneEvent, SliderInputView, CustomStructDefinition, CounterMeasurement, GaugeMeasurement, ArrowShape, Tag, Space, EffectStyle, Canvas, MaterializationType, Window, Variant, Branch, Environment } from '@/language';

import type { ArrowShape, Reaction, Friendship, NotificationEvent, Trigger, CounterMetric, Message, Invite, AnnotationShape, BorderStyle, Route, Icon, GaugeMetric, Follow, Window, Role, Layer, ThreadCursor, Run, CustomView, ResourceStatus, NumberInputView, FriendshipInvite, Machine, Variant, Action, TriggerEvent, HistogramMeasurement, SliderInputView, Environment, Log, CounterMeasurement, MembershipEvent, ThreadView, User, CustomEntityDefinition, FontStyle, TimerEvent, Field, EffectStyle, Sanction, File, Thread, CustomStructDefinition, GaugeMeasurement, LineShape, Span, Notification, Team, ColorStyle, Script, CustomViewDefinition, MaterializationType, RoleEvent, Value, InviteEvent, Handle, LabelView, FriendshipInviteEvent, Star, Timer, Service, Space, PlaneShape, Membership, Tagging, ScreenCursor, CustomEntity, CustomEnumDefinition, SplitView, RunEvent, Branch, TextView, Entitlement, Scene, WizardView, Organization, SceneEvent, Tag, Palette, Folder, EditEvent, Permission, Interruption, Database, CustomEventDefinition, Canvas, EntitlementEvent, FillStyle, HistogramMetric, ShadowStyle, Option, EventCursor, CustomEvent, Link, Theme, Agent, FrameView, TransitionStyle, Client, SanctionEvent, Snapshot, GradientStyle } from '@/language';

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
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  name: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:101 ==== */
export interface HasSlug {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  slug: string | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:102 ==== */
export interface HasIcon {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  icon: Icon | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:51 ==== */
export interface Tracked {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
export interface Visual {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:9000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50 ==== */
export interface Frozen {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:50 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:52 ==== */
export interface Archivable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly archivedAt: DateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:52 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:53 ==== */
export interface Deletable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly deletedAt: DateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:53 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:23 ==== */
export interface CustomNodeDefinition {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null;
  set prototype(value: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null): void;;
  prototypePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
export interface CustomNode {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get definition(): CustomEntityDefinition | GaugeMetric | CounterMetric | HistogramMetric | CustomViewDefinition | null;;
  readonly definitionPtr: NodeReference
}
/* ==== DESTACK_GENERATED_END:TRAIT:24 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:55 ==== */
export interface Extensible {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  value: Value;
}
/* ==== DESTACK_GENERATED_END:TRAIT:55 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:56 ==== */
export interface Ordered {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly orderKey: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:56 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5532 ==== */
export interface Reactable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:5532 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5530 ==== */
export interface Starable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:5530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5534 ==== */
export interface Followable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:5534 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3003 ==== */
export interface Sourceable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly orderKey: string;
  get source(): Script | null;
  set source(value: Script | null): void;;
  sourcePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
export interface Scriptable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get script(): Script | null;
  set script(value: Script | null): void;;
  scriptPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:3002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3001 ==== */
export interface Runnable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:3001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3000 ==== */
export interface Actionable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:3000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:500 ==== */
export interface Ownable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get ownedBy(): Role | Agent | Organization | Team | User | null;
  set ownedBy(value: Role | Agent | Organization | Team | User | null): void;;
  ownedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5000 ==== */
export interface Settings {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:5000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:502 ==== */
export interface Joinable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:502 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:505 ==== */
export interface Subject {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:505 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:506 ==== */
export interface Owner {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:506 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1000 ==== */
export interface Taggable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:1000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:510 ==== */
export interface Membership {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get member(): Agent | User | null;
  set member(value: Agent | User | null): void;;
  memberPtr: NodeReference
}
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
export interface Invite {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get member(): Agent | User | null;
  set member(value: Agent | User | null): void;;
  memberPtr: NodeReference
}
/* ==== DESTACK_GENERATED_END:TRAIT:511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1001 ==== */
export interface Tag {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:1001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5535 ==== */
export interface Follow {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:5535 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
export interface Global {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
export interface Spatial {
  readonly id: string;
  get parent(): Space | null;;
  readonly parentPtr: NodeReference | null
  get space(): Space | null;
  set space(value: Space | null): void;;
  spacePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10 ==== */
export interface Entity {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
export interface Particle {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
export interface Analytic {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
export interface Indexed {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:13 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:21 ==== */
export interface Resource {
  readonly id: string;
  get parent(): Folder | null;;
  readonly parentPtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
  status: ResourceStatus;
  targetStatus: DateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
export interface Metric {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null;
  set prototype(value: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null): void;;
  prototypePtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
  readonly orderKey: string;
  get source(): Script | null;
  set source(value: Script | null): void;;
  sourcePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
export interface Measurement {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null
  get definition(): GaugeMetric | CounterMetric | HistogramMetric | null;
  set definition(value: GaugeMetric | CounterMetric | HistogramMetric | null): void;;
  definitionPtr: NodeReference
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:4011 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:22 ==== */
export interface Event {
  readonly id: string;
  get parent(): Space | null;;
  readonly parentPtr: NodeReference | null
  get space(): Space | null;
  set space(value: Space | null): void;;
  spacePtr: NodeReference | null
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null
  get node(): Node | null;
  set node(value: Node | null): void;;
  nodePtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */