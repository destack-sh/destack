import { Icon, MaterializationType, Node, NodeReference, ResourceStatus, Script, Space, Value } from "@/language";
import { Temporal } from "temporal-polyfill";

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
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
export interface IsVisual {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
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
  get prototype(): (Node & IsCustomNode) | null | null;
  set prototype(value: (Node & IsCustomNode) | null);
  prototypePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
export interface IsCustomNode {
  readonly id: string;
  get definition(): (Node & IsCustomNodeDefinition) | null;
  readonly definitionPtr: NodeReference;
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
  get source(): Script | null | null;
  readonly sourcePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
export interface IsScriptable {
  readonly id: string;
  get script(): Script | null | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;
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
  get ownedBy(): (Node & IsOwner) | null | null;
  set ownedBy(value: (Node & IsOwner) | null);
  ownedByPtr: NodeReference | null;
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
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
export interface LikeInvite {
  readonly id: string;
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;
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
  get space(): Space | null | null;
  readonly spacePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10 ==== */
export interface Entity {
  readonly id: string;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
export interface Particle {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
export interface Analytic {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
export interface Indexed {
  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:13 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:21 ==== */
export interface Resource {
  readonly id: string;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
  status: ResourceStatus;
  targetStatus: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
export interface Metric {
  readonly id: string;
  get prototype(): (Node & IsCustomNode) | null | null;
  set prototype(value: (Node & IsCustomNode) | null);
  prototypePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
  readonly orderKey: string;
  get source(): Script | null | null;
  readonly sourcePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
export interface Measurement {
  readonly id: string;
  get definition(): (Node & Metric) | null;
  readonly definitionPtr: NodeReference;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:4011 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:22 ==== */
export interface Event {
  readonly id: string;
  get space(): Space | null | null;
  readonly spacePtr: NodeReference | null;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
  readonly updatedByPtr: NodeReference | null;
  get node(): Node | null | null;
  set node(value: Node | null);
  nodePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */
