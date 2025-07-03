import { EnumType, TraitType } from "@destack/language/core/builtin/common";
import type { CustomEntityDefinition } from "@destack/language/core/builtin/entity";
import type { CustomEventDefinition } from "@destack/language/core/builtin/event";
import { Node } from "@destack/language/core/builtin/node";
import type {
  NodeDefinitionReference,
  NodeReference,
} from "@destack/language/core/builtin/relation";
import type { PropertyDefinition, TraitDefinition } from "@destack/language/core/common/definition";
import type { Value } from "@destack/language/core/common/value";
import type { Script } from "@destack/language/logic";
import { registerEnumClass, registerTraitClass } from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/** Internal base class for Trait companion objects.*/
export class TraitClass<N = any, T extends TraitType = TraitType> {
  readonly metatype: T;
  __definition__: TraitDefinition;
  __properties__: Record<string, PropertyDefinition>;
  __propertiesById__: Record<number, PropertyDefinition>;

  constructor(metatype: any) {
    this.metatype = metatype;
    this.__definition__ = null as any; // set later (in finalize)
    this.__properties__ = {};
    this.__propertiesById__ = {};
  }

  /** Get a PropertyDefinition or CustomProperty by name. */
  property(name: string): PropertyDefinition {
    const prop = this.__properties__[name];
    if (!prop) {
      throw new Error(`Property ${name} not found on ${this.constructor.name}`);
    }
    return prop;
  }
}

/* ==== DESTACK_GENERATED_START:CONSTANT:INTER_ORDER_TYPES ==== */
/**
 * INTER_ORDER_TYPES
 */
// prettier-ignore
export const INTER_ORDER_TYPES = [
  (190400 /* NodeType.VIEW */),
  (270200 /* NodeType.STYLE */)
];

/* ==== DESTACK_GENERATED_END:CONSTANT:INTER_ORDER_TYPES ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:101 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:102 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:100 ==== */
/**
 * A Node that can be ordered.
 */
export interface IsOrdered {
  /**
   * The absolute order key of this Node in its parent.
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
/* ==== DESTACK_GENERATED_END:TRAIT:100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:120032 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:120032 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:120030 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:120030 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:120034 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:120034 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:70003 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:70003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:70002 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:70002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:70001 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:70001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:20000 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:20000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:20002 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:20002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:30000 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:30000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:110 ==== */
/**
 * A Node that can be customized with custom Properties.
 */
export interface IsCustomizable {
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  customValues: Map<string, Value>;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that can be customized with custom Properties.
 */
class IsCustomizable$Type extends TraitClass<IsCustomizable, TraitType.CUSTOMIZABLE> {}

export const IsCustomizable = new IsCustomizable$Type(TraitType.CUSTOMIZABLE);
registerTraitClass(TraitType.CUSTOMIZABLE, IsCustomizable);
/* ==== DESTACK_GENERATED_END:TRAIT:110 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:111 ==== */
/**
 * A Node that be extended by custom Nodes (i.e. used as a base type).
 */
export interface IsExtensible extends IsCustomizable {
  get definition(): CustomEntityDefinition | CustomEventDefinition | null;
  readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  readonly baseType: NodeDefinitionReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that be extended by custom Nodes (i.e. used as a base type).
 */
class IsExtensible$Type extends TraitClass<IsExtensible, TraitType.EXTENSIBLE> {}

export const IsExtensible = new IsExtensible$Type(TraitType.EXTENSIBLE);
registerTraitClass(TraitType.EXTENSIBLE, IsExtensible);
/* ==== DESTACK_GENERATED_END:TRAIT:111 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:20003 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:20003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:20001 ==== */
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
/* ==== DESTACK_GENERATED_END:TRAIT:20001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:20000 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:20000 ==== */
