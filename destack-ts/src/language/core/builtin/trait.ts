import { Icon, MaterializationType, Node, NodeReference, ResourceStatus, Value } from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:50101 ==== */
/**
 * JoinablePermission
 */
export enum JoinablePermission {
  INVITE = 1,
  REMOVE = 2,
  KICK = 3,
  BAN = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:50101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:100 ==== */
/**
 * A Node with a plain name.
 */
export interface HasName {
  /**
   * HasName.name
   */
  name: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:101 ==== */
/**
 * A Node with a slug.
 */
export interface HasSlug {
  /**
   * HasSlug.slug
   */
  slug: string | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:102 ==== */
/**
 * A Node with an icon.
 */
export interface HasIcon {
  /**
   * HasIcon.icon
   */
  icon: Icon | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:51 ==== */
/**
 * A Node that is "tracked" on create/update.
 */
export interface IsTracked {
  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  get createdBy(): (Node & IsSubject) | null;
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  get updatedBy(): (Node & IsSubject) | null;
  readonly updatedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
/**
 * A Node that is a visual in some sense (views, styles, drawings, ...).
 */
export interface IsVisual extends IsTracked {}
/* ==== DESTACK_GENERATED_END:TRAIT:9000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50 ==== */
/**
 * A Node that is frozen (read-only).
 * TODO :Cleanup: Nodes don't set 'real' frozen=True (like StructFrozen) :PretendFrozen
 *  (because that would require two separate inheritance chains for NodeMutable and NodeFrozen,
 *   which would have to include copies of every relevant trait and .. ughh no)
 */
export interface IsFrozen {}
/* ==== DESTACK_GENERATED_END:TRAIT:50 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:52 ==== */
/**
 * A Node that can be archived.
 */
export interface IsArchivable {
  /**
   * IsArchivable.archivedAt
   */
  readonly archivedAt: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:52 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:53 ==== */
/**
 * A Node that can be deleted.
 */
export interface IsDeletable {
  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:53 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:23 ==== */
/**
 * A Node that defines a Custom Node type.
 */
export interface IsCustomNodeDefinition {
  get prototype(): (Node & IsCustomNode) | null;
  set prototype(value: (Node & IsCustomNode) | null);
  prototypePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
/**
 * A Node that is asome Custom Node.
 */
export interface IsCustomNode {
  get definition(): (Node & IsCustomNodeDefinition) | null;
  readonly definitionPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:24 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:55 ==== */
/**
 * A Node that can be extended with custom Values (one Value per Field).
 */
export interface IsExtensible {
  /**
   * IsExtensible.value
   */
  value: Map<string, Value>;
}
/* ==== DESTACK_GENERATED_END:TRAIT:55 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:56 ==== */
/**
 * A Node that can be ordered.
 */
export interface IsOrdered {
  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:56 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5532 ==== */
/**
 * A Node that can be reacted to (with Reactions).
 */
export interface IsReactable {}
/* ==== DESTACK_GENERATED_END:TRAIT:5532 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5530 ==== */
/**
 * A Node that can be starred (with Stars).
 */
export interface IsStarable {}
/* ==== DESTACK_GENERATED_END:TRAIT:5530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5534 ==== */
/**
 * A Node that can be followed (with Follows).
 */
export interface IsFollowable {}
/* ==== DESTACK_GENERATED_END:TRAIT:5534 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3003 ==== */
/**
 * A Node that can be sourced from / defined by a Script.
 */
export interface IsSourceable extends IsOrdered {
  get source(): Script | null;
  readonly sourcePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
/**
 * A Node that can be scripted.
 */
export interface IsScriptable {
  get script(): Script | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:3002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3001 ==== */
/**
 * A Node that can be run (with Runs).
 */
export interface IsRunnable {}
/* ==== DESTACK_GENERATED_END:TRAIT:3001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3000 ==== */
/**
 * A Node that can define an Action.
 */
export interface IsActionable {}
/* ==== DESTACK_GENERATED_END:TRAIT:3000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:500 ==== */
/**
 * A Node that can be owned by another Node.
 */
export interface IsOwnable {
  get ownedBy(): (Node & IsOwner) | null;
  set ownedBy(value: (Node & IsOwner) | null);
  ownedByPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5000 ==== */
/**
 * A Node that defines Settings.
 */
export interface IsSettings {}
/* ==== DESTACK_GENERATED_END:TRAIT:5000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:502 ==== */
/**
 * A Node that can be joined by Subjects.
 */
export interface IsJoinable {}
/* ==== DESTACK_GENERATED_END:TRAIT:502 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:505 ==== */
/**
 * A Node that can be a Subject.
 */
export interface IsSubject {}
/* ==== DESTACK_GENERATED_END:TRAIT:505 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:506 ==== */
/**
 * A Node that can be an Owner.
 */
export interface IsOwner {}
/* ==== DESTACK_GENERATED_END:TRAIT:506 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1000 ==== */
/**
 * A Node that can be tagged (with a Tag).
 */
export interface IsTaggable {}
/* ==== DESTACK_GENERATED_END:TRAIT:1000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:510 ==== */
/**
 * A Node that represents a Membership.
 */
export interface LikeMembership {
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
/**
 * A Node that represents an Invite.
 */
export interface LikeInvite {
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1001 ==== */
/**
 * A Node that represents a Tag.
 */
export interface LikeTag {}
/* ==== DESTACK_GENERATED_END:TRAIT:1001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5535 ==== */
/**
 * A Node that represents a Follow.
 */
export interface LikeFollow {}
/* ==== DESTACK_GENERATED_END:TRAIT:5535 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
/**
 * A Node that is global.
 */
export interface Global {}
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
/**
 * A Node in a Space.
 */
export interface Spatial {
  get space(): Space | null;
  readonly spacePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10 ==== */
/**
 * An Entity is a versioned Node in primary relational storage (OLTP).
 */
export interface Entity extends IsTracked {
  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;
}
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
/**
 * A Particle is a forward-only Node in primary document storage (OLTP, high volume).
 */
export interface Particle extends IsTracked {}
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
/**
 * An Analytic is a read-only Node in primary or secondary warehouse storage (OLAP, bulk).
 */
export interface Analytic extends IsTracked {}
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
/**
 * A Node that is indexed in secondary search storage (OLTP).
 */
export interface Indexed extends IsTracked {}
/* ==== DESTACK_GENERATED_END:TRAIT:13 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:21 ==== */
/**
 * A Resource represents an external asset.
 * The lifecycle of a Resource may be managed by some provisioner.
 */
export interface Resource extends Entity {
  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
/**
 * An Entity that represents a Metric.
 */
export interface Metric extends Entity, IsCustomNodeDefinition, IsSourceable {}
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
/**
 * An Analytic that represents a Measurement.
 */
export interface Measurement extends Analytic, IsCustomNode {
  get definition(): (Node & Metric) | null;
  readonly definitionPtr: NodeReference;
}
/* ==== DESTACK_GENERATED_END:TRAIT:4011 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:22 ==== */
/**
 * An Event is a Node that represents an Event.
 * Events always belong to a specific Space.
 */
export interface Event extends Spatial, Particle, Analytic, Indexed, IsFrozen {
  get node(): Node | null;
  set node(value: Node | null);
  nodePtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */
