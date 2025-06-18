import type { Space } from '@/language/space/space.ts';
import type { Handle } from '@/language/space/handle.ts';
import type { User } from '@/language/space/user.ts';
import type { Friendship, FriendshipInvite, FriendshipInviteEvent } from '@/language/space/friendship.ts';
import type { Organization } from '@/language/space/organization.ts';
import type { Team } from '@/language/space/team.ts';
import type { Client } from '@/language/space/client.ts';
import type { Membership, MembershipEvent } from '@/language/access/membership.ts';
import type { Invite, InviteEvent } from '@/language/access/invite.ts';
import type { Role, RoleEvent } from '@/language/access/role.ts';
import type { Permission } from '@/language/access/permission.ts';
import type { Sanction, SanctionEvent } from '@/language/access/sanction.ts';
import type { Entitlement, EntitlementEvent } from '@/language/access/entitlement.ts';
import type { Agent } from '@/language/space/agent.ts';
import type { Folder } from '@/language/folder/folder.ts';
import type { Tag, Tagging } from '@/language/folder/tag.ts';
import type { Snapshot, Branch } from '@/language/core/common/spacetime.ts';
import type { CustomEntityDefinition, CustomEntity } from '@/language/core/common/entity.ts';
import type { CustomStructDefinition } from '@/language/core/common/struct.ts';
import type { CustomEnumDefinition } from '@/language/core/common/enum.ts';
import type { Field } from '@/language/core/common/field.ts';
import type { Option } from '@/language/core/common/option.ts';
import type { File } from '@/language/data/file.ts';
import type { Link } from '@/language/data/link.ts';
import type { Script } from '@/language/logic/script.ts';
import type { Service } from '@/language/logic/service.ts';
import type { Action } from '@/language/logic/action.ts';
import type { Route } from '@/language/logic/route.ts';
import type { Trigger, TriggerEvent } from '@/language/logic/trigger.ts';
import type { Timer, TimerEvent } from '@/language/logic/timer.ts';
import type { EventCursor, ScreenCursor, ThreadCursor } from '@/language/logic/cursor.ts';
import type { Run, RunEvent } from '@/language/runtime/run.ts';
import type { Span } from '@/language/runtime/span.ts';
import type { Interruption } from '@/language/runtime/interruption.ts';
import type { Log } from '@/language/runtime/log.ts';
import type { GaugeMetric, GaugeMeasurement, CounterMetric, CounterMeasurement, HistogramMetric, HistogramMeasurement } from '@/language/core/common/metric.ts';
import type { CustomEventDefinition, CustomEvent, EditEvent } from '@/language/core/common/event.ts';
import type { Environment } from '@/language/deployment/environment.ts';
import type { Thread } from '@/language/social/thread.ts';
import type { Message } from '@/language/social/message.ts';
import type { Reaction } from '@/language/social/reaction.ts';
import type { Star } from '@/language/social/star.ts';
import type { Follow } from '@/language/social/follow.ts';
import type { Notification, NotificationEvent } from '@/language/social/notification.ts';
import type { Database } from '@/language/infra/database.ts';
import type { Machine } from '@/language/infra/machine.ts';
import type { Window } from '@/language/scene/window.ts';
import type { Scene, SceneEvent } from '@/language/scene/scene.ts';
import type { Layer } from '@/language/scene/layer.ts';
import type { Variant } from '@/language/scene/variant.ts';
import type { CustomViewDefinition, CustomView } from '@/language/view/container/custom.ts';
import type { FrameView } from '@/language/view/container/frame.ts';
import type { LabelView } from '@/language/view/container/label.ts';
import type { SplitView } from '@/language/view/container/split.ts';
import type { TextView } from '@/language/view/content/text.ts';
import type { NumberInputView } from '@/language/view/input/number.ts';
import type { SliderInputView } from '@/language/view/input/slider.ts';
import type { ThreadView } from '@/language/view/node/thread.ts';
import type { WizardView } from '@/language/view/internal/wizard.ts';
import type { Canvas } from '@/language/canvas/canvas.ts';
import type { LineShape } from '@/language/canvas/line.ts';
import type { PlaneShape } from '@/language/canvas/plane.ts';
import type { ArrowShape } from '@/language/canvas/arrow.ts';
import type { AnnotationShape } from '@/language/canvas/annotation.ts';
import type { Theme } from '@/language/style/theme.ts';
import type { Palette } from '@/language/style/palette.ts';
import type { ColorStyle } from '@/language/style/color.ts';
import type { FillStyle } from '@/language/style/fill.ts';
import type { FontStyle } from '@/language/style/font.ts';
import type { BorderStyle } from '@/language/style/border.ts';
import type { ShadowStyle } from '@/language/style/shadow.ts';
import type { GradientStyle } from '@/language/style/gradient.ts';
import type { TransitionStyle } from '@/language/style/transition.ts';
import type { EffectStyle } from '@/language/style/effect.ts';
import type { Icon } from '@/language/core/common/icon.ts';
import type { Value } from '@/language/core/common/value.ts';
import type { MaterializationType, ResourceStatus } from '@/language/core/builtin/common.ts';

import { DateTime } from "@/proto";

export interface Trait {

}

export interface IsOwnable extends Trait {

}

export interface IsDeletable extends Trait {
    deletedAt: DateTime | null;
}

export interface IsArchivable extends Trait {
	archivedAt: DateTime | null;
}

export interface IsTracked extends Trait {
	createdAt: DateTime;
	createdByPtr: NodeReference | null;
	get createdBy(): IsOwnable | null;
	set createdBy(value: IsOwnable | null);
	updatedAt: DateTime;
	updatedByPtr: NodeReference | null;
	get updatedBy(): IsOwnable | null;
	set updatedBy(value: IsOwnable | null);
}

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
  readonly parentPtr: NodeReference | null;
  name: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:101 ==== */
export interface HasSlug {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  slug: string | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:102 ==== */
export interface HasIcon {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  icon: Icon | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:51 ==== */
export interface Tracked {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
export interface Visual {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:9000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50 ==== */
export interface Frozen {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:50 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:52 ==== */
export interface Archivable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly archivedAt: DateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:52 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:53 ==== */
export interface Deletable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly deletedAt: DateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:53 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:23 ==== */
export interface CustomNodeDefinition {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null;
  set prototype(value: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null): void;;
  prototypePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
export interface CustomNode {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get definition(): CustomEntityDefinition | GaugeMetric | CounterMetric | HistogramMetric | CustomViewDefinition | null;;
  readonly definitionPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:24 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:55 ==== */
export interface Extensible {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  value: Value;
}
/* ==== DESTACK_GENERATED_END:TRAIT:55 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:56 ==== */
export interface Ordered {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly orderKey: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:56 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5532 ==== */
export interface Reactable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5532 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5530 ==== */
export interface Starable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5534 ==== */
export interface Followable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5534 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3003 ==== */
export interface Sourceable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly orderKey: string;
  get source(): Script | null;
  set source(value: Script | null): void;;
  sourcePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
export interface Scriptable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get script(): Script | null;
  set script(value: Script | null): void;;
  scriptPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3001 ==== */
export interface Runnable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3000 ==== */
export interface Actionable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:500 ==== */
export interface Ownable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null;
  set ownedBy(value: Role | Agent | Organization | Team | User | null): void;;
  ownedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5000 ==== */
export interface Settings {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:502 ==== */
export interface Joinable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:502 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:505 ==== */
export interface Subject {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:505 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:506 ==== */
export interface Owner {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:506 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1000 ==== */
export interface Taggable {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:1000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:510 ==== */
export interface Membership {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get member(): Agent | User | null;
  set member(value: Agent | User | null): void;;
  memberPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
export interface Invite {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get member(): Agent | User | null;
  set member(value: Agent | User | null): void;;
  memberPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1001 ==== */
export interface Tag {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:1001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5535 ==== */
export interface Follow {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:5535 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
export interface Global {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
export interface Spatial {
  readonly id: string;
  get parent(): Space | null;;
  readonly parentPtr: NodeReference | null;
  get space(): Space | null;
  set space(value: Space | null): void;;
  spacePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10 ==== */
export interface Entity {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  materialization: MaterializationType;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
export interface Particle {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
export interface Analytic {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
export interface Indexed {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:13 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:21 ==== */
export interface Resource {
  readonly id: string;
  get parent(): Folder | null;;
  readonly parentPtr: NodeReference | null;
  materialization: MaterializationType;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
  status: ResourceStatus;
  targetStatus: DateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
export interface Metric {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get prototype(): CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null;
  set prototype(value: CustomEntity | GaugeMeasurement | CounterMeasurement | HistogramMeasurement | CustomView | null): void;;
  prototypePtr: NodeReference | null;
  materialization: MaterializationType;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
  readonly orderKey: string;
  get source(): Script | null;
  set source(value: Script | null): void;;
  sourcePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
export interface Measurement {
  readonly id: string;
  get parent(): Node | null;;
  readonly parentPtr: NodeReference | null;
  get definition(): GaugeMetric | CounterMetric | HistogramMetric | null;
  set definition(value: GaugeMetric | CounterMetric | HistogramMetric | null): void;;
  definitionPtr: NodeReference;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:4011 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:22 ==== */
export interface Event {
  readonly id: string;
  get parent(): Space | null;;
  readonly parentPtr: NodeReference | null;
  get space(): Space | null;
  set space(value: Space | null): void;;
  spacePtr: NodeReference | null;
  readonly createdAt: DateTime;
  get createdBy(): Agent | User | null;;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: DateTime;
  get updatedBy(): Agent | User | null;;
  readonly updatedByPtr: NodeReference | null;
  get node(): Node | null;
  set node(value: Node | null): void;;
  nodePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */