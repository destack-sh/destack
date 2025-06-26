import { NodeReference } from "@destack/language/core";
import {
  EnumType,
  MaterializationType,
  Node,
  ResourceStatus,
  TraitClass,
  TraitType,
} from "@destack/language/core/builtin";
import { Icon, Value } from "@destack/language/core/common";
import { Script } from "@destack/language/logic";
import { registerEnumClass, registerTraitClass } from "@destack/language/registry";
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.JOINABLE_PERMISSION, JoinablePermission);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node with a plain name.
 */
class HasName$Type extends TraitClass<HasName, TraitType.HAS_NAME> {}

export const HasName = new HasName$Type(TraitType.HAS_NAME);
registerTraitClass(TraitType.HAS_NAME, HasName);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node with a slug.
 */
class HasSlug$Type extends TraitClass<HasSlug, TraitType.HAS_SLUG> {}

export const HasSlug = new HasSlug$Type(TraitType.HAS_SLUG);
registerTraitClass(TraitType.HAS_SLUG, HasSlug);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node with an icon.
 */
class HasIcon$Type extends TraitClass<HasIcon, TraitType.HAS_ICON> {}

export const HasIcon = new HasIcon$Type(TraitType.HAS_ICON);
registerTraitClass(TraitType.HAS_ICON, HasIcon);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is "tracked" on create/update.
 */
class IsTracked$Type extends TraitClass<IsTracked, TraitType.TRACKED> {}

export const IsTracked = new IsTracked$Type(TraitType.TRACKED);
registerTraitClass(TraitType.TRACKED, IsTracked);
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
/**
 * A Node that is a visual in some sense (views, styles, drawings, ...).
 */
export interface IsVisual extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is a visual in some sense (views, styles, drawings, ...).
 */
class IsVisual$Type extends TraitClass<IsVisual, TraitType.VISUAL> {}

export const IsVisual = new IsVisual$Type(TraitType.VISUAL);
registerTraitClass(TraitType.VISUAL, IsVisual);
/* ==== DESTACK_GENERATED_END:TRAIT:9000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50 ==== */
/**
 * A Node that is frozen (read-only).
 */
export interface IsFrozen {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is frozen (read-only).
 */
class IsFrozen$Type extends TraitClass<IsFrozen, TraitType.FROZEN> {}

export const IsFrozen = new IsFrozen$Type(TraitType.FROZEN);
registerTraitClass(TraitType.FROZEN, IsFrozen);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be archived.
 */
class IsArchivable$Type extends TraitClass<IsArchivable, TraitType.ARCHIVABLE> {}

export const IsArchivable = new IsArchivable$Type(TraitType.ARCHIVABLE);
registerTraitClass(TraitType.ARCHIVABLE, IsArchivable);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be deleted.
 */
class IsDeletable$Type extends TraitClass<IsDeletable, TraitType.DELETABLE> {}

export const IsDeletable = new IsDeletable$Type(TraitType.DELETABLE);
registerTraitClass(TraitType.DELETABLE, IsDeletable);
/* ==== DESTACK_GENERATED_END:TRAIT:53 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:23 ==== */
/**
 * A Node that defines a Custom Node type.
 */
export interface IsCustomNodeDefinition {
  get prototype(): (Node & IsCustomNode) | null;
  set prototype(value: (Node & IsCustomNode) | null);
  prototypePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that defines a Custom Node type.
 */
class IsCustomNodeDefinition$Type extends TraitClass<IsCustomNodeDefinition, TraitType.CUSTOM_NODE_DEFINITION> {}

export const IsCustomNodeDefinition = new IsCustomNodeDefinition$Type(TraitType.CUSTOM_NODE_DEFINITION);
registerTraitClass(TraitType.CUSTOM_NODE_DEFINITION, IsCustomNodeDefinition);
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
/**
 * A Node that is asome Custom Node.
 */
export interface IsCustomNode {
  get definition(): (Node & IsCustomNodeDefinition) | null;
  readonly definitionPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is asome Custom Node.
 */
class IsCustomNode$Type extends TraitClass<IsCustomNode, TraitType.CUSTOM_NODE> {}

export const IsCustomNode = new IsCustomNode$Type(TraitType.CUSTOM_NODE);
registerTraitClass(TraitType.CUSTOM_NODE, IsCustomNode);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be extended with custom Values (one Value per Field).
 */
class IsExtensible$Type extends TraitClass<IsExtensible, TraitType.EXTENSIBLE> {}

export const IsExtensible = new IsExtensible$Type(TraitType.EXTENSIBLE);
registerTraitClass(TraitType.EXTENSIBLE, IsExtensible);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be ordered.
 */
class IsOrdered$Type extends TraitClass<IsOrdered, TraitType.ORDERED> {}

export const IsOrdered = new IsOrdered$Type(TraitType.ORDERED);
registerTraitClass(TraitType.ORDERED, IsOrdered);
/* ==== DESTACK_GENERATED_END:TRAIT:56 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5532 ==== */
/**
 * A Node that can be reacted to (with Reactions).
 */
export interface IsReactable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be reacted to (with Reactions).
 */
class IsReactable$Type extends TraitClass<IsReactable, TraitType.REACTABLE> {}

export const IsReactable = new IsReactable$Type(TraitType.REACTABLE);
registerTraitClass(TraitType.REACTABLE, IsReactable);
/* ==== DESTACK_GENERATED_END:TRAIT:5532 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5530 ==== */
/**
 * A Node that can be starred (with Stars).
 */
export interface IsStarable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be starred (with Stars).
 */
class IsStarable$Type extends TraitClass<IsStarable, TraitType.STARABLE> {}

export const IsStarable = new IsStarable$Type(TraitType.STARABLE);
registerTraitClass(TraitType.STARABLE, IsStarable);
/* ==== DESTACK_GENERATED_END:TRAIT:5530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5534 ==== */
/**
 * A Node that can be followed (with Follows).
 */
export interface IsFollowable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be followed (with Follows).
 */
class IsFollowable$Type extends TraitClass<IsFollowable, TraitType.FOLLOWABLE> {}

export const IsFollowable = new IsFollowable$Type(TraitType.FOLLOWABLE);
registerTraitClass(TraitType.FOLLOWABLE, IsFollowable);
/* ==== DESTACK_GENERATED_END:TRAIT:5534 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3003 ==== */
/**
 * A Node that can be sourced from / defined by a Script.
 */
export interface IsSourceable extends IsOrdered {
  get source(): Script | null;
  readonly sourcePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be sourced from / defined by a Script.
 */
class IsSourceable$Type extends TraitClass<IsSourceable, TraitType.SOURCEABLE> {}

export const IsSourceable = new IsSourceable$Type(TraitType.SOURCEABLE);
registerTraitClass(TraitType.SOURCEABLE, IsSourceable);
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
/**
 * A Node that can be scripted.
 */
export interface IsScriptable {
  get script(): Script | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be scripted.
 */
class IsScriptable$Type extends TraitClass<IsScriptable, TraitType.SCRIPTABLE> {}

export const IsScriptable = new IsScriptable$Type(TraitType.SCRIPTABLE);
registerTraitClass(TraitType.SCRIPTABLE, IsScriptable);
/* ==== DESTACK_GENERATED_END:TRAIT:3002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3001 ==== */
/**
 * A Node that can be run (with Runs).
 */
export interface IsRunnable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be run (with Runs).
 */
class IsRunnable$Type extends TraitClass<IsRunnable, TraitType.RUNNABLE> {}

export const IsRunnable = new IsRunnable$Type(TraitType.RUNNABLE);
registerTraitClass(TraitType.RUNNABLE, IsRunnable);
/* ==== DESTACK_GENERATED_END:TRAIT:3001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3000 ==== */
/**
 * A Node that can define an Action.
 */
export interface IsActionable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can define an Action.
 */
class IsActionable$Type extends TraitClass<IsActionable, TraitType.ACTIONABLE> {}

export const IsActionable = new IsActionable$Type(TraitType.ACTIONABLE);
registerTraitClass(TraitType.ACTIONABLE, IsActionable);
/* ==== DESTACK_GENERATED_END:TRAIT:3000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:500 ==== */
/**
 * A Node that can be owned by another Node.
 */
export interface IsOwnable {
  get ownedBy(): (Node & IsOwner) | null;
  set ownedBy(value: (Node & IsOwner) | null);
  ownedByPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be owned by another Node.
 */
class IsOwnable$Type extends TraitClass<IsOwnable, TraitType.OWNABLE> {}

export const IsOwnable = new IsOwnable$Type(TraitType.OWNABLE);
registerTraitClass(TraitType.OWNABLE, IsOwnable);
/* ==== DESTACK_GENERATED_END:TRAIT:500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5000 ==== */
/**
 * A Node that defines Settings.
 */
export interface IsSettings {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that defines Settings.
 */
class IsSettings$Type extends TraitClass<IsSettings, TraitType.SETTINGS> {}

export const IsSettings = new IsSettings$Type(TraitType.SETTINGS);
registerTraitClass(TraitType.SETTINGS, IsSettings);
/* ==== DESTACK_GENERATED_END:TRAIT:5000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:502 ==== */
/**
 * A Node that can be joined by Subjects.
 */
export interface IsJoinable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be joined by Subjects.
 */
class IsJoinable$Type extends TraitClass<IsJoinable, TraitType.JOINABLE> {}

export const IsJoinable = new IsJoinable$Type(TraitType.JOINABLE);
registerTraitClass(TraitType.JOINABLE, IsJoinable);
/* ==== DESTACK_GENERATED_END:TRAIT:502 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:505 ==== */
/**
 * A Node that can be a Subject.
 */
export interface IsSubject {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be a Subject.
 */
class IsSubject$Type extends TraitClass<IsSubject, TraitType.SUBJECT> {}

export const IsSubject = new IsSubject$Type(TraitType.SUBJECT);
registerTraitClass(TraitType.SUBJECT, IsSubject);
/* ==== DESTACK_GENERATED_END:TRAIT:505 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:506 ==== */
/**
 * A Node that can be an Owner.
 */
export interface IsOwner {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be an Owner.
 */
class IsOwner$Type extends TraitClass<IsOwner, TraitType.OWNER> {}

export const IsOwner = new IsOwner$Type(TraitType.OWNER);
registerTraitClass(TraitType.OWNER, IsOwner);
/* ==== DESTACK_GENERATED_END:TRAIT:506 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1000 ==== */
/**
 * A Node that can be tagged (with a Tag).
 */
export interface IsTaggable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be tagged (with a Tag).
 */
class IsTaggable$Type extends TraitClass<IsTaggable, TraitType.TAGGABLE> {}

export const IsTaggable = new IsTaggable$Type(TraitType.TAGGABLE);
registerTraitClass(TraitType.TAGGABLE, IsTaggable);
/* ==== DESTACK_GENERATED_END:TRAIT:1000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:510 ==== */
/**
 * A Node that represents a Membership.
 */
export interface LikeMembership {
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that represents a Membership.
 */
class LikeMembership$Type extends TraitClass<LikeMembership, TraitType.MEMBERSHIP> {}

export const LikeMembership = new LikeMembership$Type(TraitType.MEMBERSHIP);
registerTraitClass(TraitType.MEMBERSHIP, LikeMembership);
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
/**
 * A Node that represents an Invite.
 */
export interface LikeInvite {
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that represents an Invite.
 */
class LikeInvite$Type extends TraitClass<LikeInvite, TraitType.INVITE> {}

export const LikeInvite = new LikeInvite$Type(TraitType.INVITE);
registerTraitClass(TraitType.INVITE, LikeInvite);
/* ==== DESTACK_GENERATED_END:TRAIT:511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1001 ==== */
/**
 * A Node that represents a Tag.
 */
export interface LikeTag {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that represents a Tag.
 */
class LikeTag$Type extends TraitClass<LikeTag, TraitType.TAG> {}

export const LikeTag = new LikeTag$Type(TraitType.TAG);
registerTraitClass(TraitType.TAG, LikeTag);
/* ==== DESTACK_GENERATED_END:TRAIT:1001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5535 ==== */
/**
 * A Node that represents a Follow.
 */
export interface LikeFollow {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that represents a Follow.
 */
class LikeFollow$Type extends TraitClass<LikeFollow, TraitType.FOLLOW> {}

export const LikeFollow = new LikeFollow$Type(TraitType.FOLLOW);
registerTraitClass(TraitType.FOLLOW, LikeFollow);
/* ==== DESTACK_GENERATED_END:TRAIT:5535 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
/**
 * A Node that is global.
 */
export interface Global {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is global.
 */
class Global$Type extends TraitClass<Global, TraitType.GLOBAL> {}

export const Global = new Global$Type(TraitType.GLOBAL);
registerTraitClass(TraitType.GLOBAL, Global);
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
/**
 * A Node in a Space.
 */
export interface Spatial {
  get space(): Space | null;
  readonly spacePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node in a Space.
 */
class Spatial$Type extends TraitClass<Spatial, TraitType.SPATIAL> {}

export const Spatial = new Spatial$Type(TraitType.SPATIAL);
registerTraitClass(TraitType.SPATIAL, Spatial);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity is a versioned Node in primary relational storage (OLTP).
 */
class Entity$Type extends TraitClass<Entity, TraitType.ENTITY> {}

export const Entity = new Entity$Type(TraitType.ENTITY);
registerTraitClass(TraitType.ENTITY, Entity);
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
/**
 * A Particle is a forward-only Node in primary document storage (OLTP, high volume).
 */
export interface Particle extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Particle is a forward-only Node in primary document storage (OLTP, high volume).
 */
class Particle$Type extends TraitClass<Particle, TraitType.PARTICLE> {}

export const Particle = new Particle$Type(TraitType.PARTICLE);
registerTraitClass(TraitType.PARTICLE, Particle);
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
/**
 * An Analytic is a read-only Node in primary or secondary warehouse storage (OLAP, bulk).
 */
export interface Analytic extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Analytic is a read-only Node in primary or secondary warehouse storage (OLAP, bulk).
 */
class Analytic$Type extends TraitClass<Analytic, TraitType.ANALYTIC> {}

export const Analytic = new Analytic$Type(TraitType.ANALYTIC);
registerTraitClass(TraitType.ANALYTIC, Analytic);
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
/**
 * A Node that is indexed in secondary search storage (OLTP).
 */
export interface Indexed extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is indexed in secondary search storage (OLTP).
 */
class Indexed$Type extends TraitClass<Indexed, TraitType.INDEXED> {}

export const Indexed = new Indexed$Type(TraitType.INDEXED);
registerTraitClass(TraitType.INDEXED, Indexed);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Resource represents an external asset.
 * The lifecycle of a Resource may be managed by some provisioner.
 */
class Resource$Type extends TraitClass<Resource, TraitType.RESOURCE> {}

export const Resource = new Resource$Type(TraitType.RESOURCE);
registerTraitClass(TraitType.RESOURCE, Resource);
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
/**
 * An Entity that represents a Metric.
 */
export interface Metric extends Entity, IsCustomNodeDefinition, IsSourceable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that represents a Metric.
 */
class Metric$Type extends TraitClass<Metric, TraitType.METRIC> {}

export const Metric = new Metric$Type(TraitType.METRIC);
registerTraitClass(TraitType.METRIC, Metric);
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
/**
 * An Analytic that represents a Measurement.
 */
export interface Measurement extends Analytic, IsCustomNode {
  get definition(): (Node & Metric) | null;
  readonly definitionPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Analytic that represents a Measurement.
 */
class Measurement$Type extends TraitClass<Measurement, TraitType.MEASUREMENT> {}

export const Measurement = new Measurement$Type(TraitType.MEASUREMENT);
registerTraitClass(TraitType.MEASUREMENT, Measurement);
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Event is a Node that represents an Event.
 * Events always belong to a specific Space.
 */
class Event$Type extends TraitClass<Event, TraitType.EVENT> {}

export const Event = new Event$Type(TraitType.EVENT);
registerTraitClass(TraitType.EVENT, Event);
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */

/* ==== DESTACK_GENERATED_START:CONSTANT:INTER_ORDER_TRAITS ==== */
/**
 * INTER_ORDER_TRAITS
 */
// prettier-ignore
export const INTER_ORDER_TRAITS = [
  TraitType.VIEW,
  TraitType.STYLE
];

/* ==== DESTACK_GENERATED_END:CONSTANT:INTER_ORDER_TRAITS ==== */
