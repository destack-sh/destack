import { NodeReference } from "@destack/language/core";
import { EnumType, Node, NodeType, TraitClass, TraitType } from "@destack/language/core/builtin";
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

/* ==== DESTACK_GENERATED_START:TRAIT:50200 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50200 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50201 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50201 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50202 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50202 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50100 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50101 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50102 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50103 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50103 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50104 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:50104 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50001 ==== */
/**
 * A Node that is global.
 */
export interface IsGlobal {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is global.
 */
class IsGlobal$Type extends TraitClass<IsGlobal, TraitType.GLOBAL> {}

export const IsGlobal = new IsGlobal$Type(TraitType.GLOBAL);
registerTraitClass(TraitType.GLOBAL, IsGlobal);
/* ==== DESTACK_GENERATED_END:TRAIT:50001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50002 ==== */
/**
 * A Node in a Space.
 */
export interface IsSpatial {
  get space(): Space | null;
  readonly spacePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node in a Space.
 */
class IsSpatial$Type extends TraitClass<IsSpatial, TraitType.SPATIAL> {}

export const IsSpatial = new IsSpatial$Type(TraitType.SPATIAL);
registerTraitClass(TraitType.SPATIAL, IsSpatial);
/* ==== DESTACK_GENERATED_END:TRAIT:50002 ==== */

/* ==== DESTACK_GENERATED_START:CONSTANT:INTER_ORDER_TYPES ==== */
/**
 * INTER_ORDER_TYPES
 */
// prettier-ignore
export const INTER_ORDER_TYPES = [
  NodeType.VIEW,
  NodeType.STYLE
];

/* ==== DESTACK_GENERATED_END:CONSTANT:INTER_ORDER_TYPES ==== */
