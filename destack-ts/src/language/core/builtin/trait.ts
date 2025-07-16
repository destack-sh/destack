import { TraitType } from "@destack/language/core/builtin/common";
import { Entity } from "@destack/language/core/builtin/entity";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { PropertyDefinition, TraitDefinition } from "@destack/language/core/common/definition";
import type { Value } from "@destack/language/core/common/value";
import type { Script } from "@destack/language/logic";
import { registerTraitClass } from "@destack/language/registry";

/** Internal base class for Trait companion objects.*/
export class TraitClass<N = any, T extends TraitType = TraitType> {
  readonly metatype: T;
  __definition__: TraitDefinition;
  __properties__: Record<string, PropertyDefinition>;
  __propertiesByAlias__: Record<string, PropertyDefinition>;
  __propertiesById__: Record<number, PropertyDefinition>;

  constructor(metatype: any) {
    this.metatype = metatype;
    this.__definition__ = null as any; // set later (in finalize)
    this.__properties__ = {};
    this.__propertiesByAlias__ = {};
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

/* ==== DESTACK_GENERATED_START:TRAIT:300000 ==== */
/**
 * An Entity that can be owned by an Actor.
 */
export interface IsOwnable {
  get ownedBy(): (Entity & IsActor) | null;
  set ownedBy(value: (Entity & IsActor) | null);
  /**
   * IsOwnable.ownedBy
   */
  get ownedByPtr(): NodeReference | null;
  set ownedByPtr(value: NodeReference | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be owned by an Actor.
 */
class IsOwnable$Type extends TraitClass<IsOwnable, TraitType.OWNABLE> {}

export const IsOwnable = new IsOwnable$Type(TraitType.OWNABLE);
registerTraitClass(TraitType.OWNABLE, IsOwnable);
/* ==== DESTACK_GENERATED_END:TRAIT:300000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:300001 ==== */
/**
 * An Entity that must be owned by an Actor.
 */
export interface IsOwned extends IsOwnable {
  get ownedBy(): (Entity & IsActor) | null;
  set ownedBy(value: Entity & IsActor);
  /**
   * IsOwned.ownedBy
   */
  get ownedByPtr(): NodeReference;
  set ownedByPtr(value: NodeReference);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that must be owned by an Actor.
 */
class IsOwned$Type extends TraitClass<IsOwned, TraitType.OWNED> {}

export const IsOwned = new IsOwned$Type(TraitType.OWNED);
registerTraitClass(TraitType.OWNED, IsOwned);
/* ==== DESTACK_GENERATED_END:TRAIT:300001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:300003 ==== */
/**
 * An Entity that can be joined by Actors.
 */
export interface IsJoinable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be joined by Actors.
 */
class IsJoinable$Type extends TraitClass<IsJoinable, TraitType.JOINABLE> {}

export const IsJoinable = new IsJoinable$Type(TraitType.JOINABLE);
registerTraitClass(TraitType.JOINABLE, IsJoinable);
/* ==== DESTACK_GENERATED_END:TRAIT:300003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:300004 ==== */
/**
 * An Entity that can be an Actor (can do something).
 */
export interface IsActor {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be an Actor (can do something).
 */
class IsActor$Type extends TraitClass<IsActor, TraitType.ACTOR> {}

export const IsActor = new IsActor$Type(TraitType.ACTOR);
registerTraitClass(TraitType.ACTOR, IsActor);
/* ==== DESTACK_GENERATED_END:TRAIT:300004 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:201000 ==== */
/**
 * An Entity that can be tagged (with a Tag).
 */
export interface IsTaggable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be tagged (with a Tag).
 */
class IsTaggable$Type extends TraitClass<IsTaggable, TraitType.TAGGABLE> {}

export const IsTaggable = new IsTaggable$Type(TraitType.TAGGABLE);
registerTraitClass(TraitType.TAGGABLE, IsTaggable);
/* ==== DESTACK_GENERATED_END:TRAIT:201000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1400032 ==== */
/**
 * An Entity that can be reacted to (with Reactions).
 */
export interface IsReactable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be reacted to (with Reactions).
 */
class IsReactable$Type extends TraitClass<IsReactable, TraitType.REACTABLE> {}

export const IsReactable = new IsReactable$Type(TraitType.REACTABLE);
registerTraitClass(TraitType.REACTABLE, IsReactable);
/* ==== DESTACK_GENERATED_END:TRAIT:1400032 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1400030 ==== */
/**
 * An Entity that can be starred (with Stars).
 */
export interface IsStarable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be starred (with Stars).
 */
class IsStarable$Type extends TraitClass<IsStarable, TraitType.STARABLE> {}

export const IsStarable = new IsStarable$Type(TraitType.STARABLE);
registerTraitClass(TraitType.STARABLE, IsStarable);
/* ==== DESTACK_GENERATED_END:TRAIT:1400030 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1400034 ==== */
/**
 * An Entity that can be followed (with Follows).
 */
export interface IsFollowable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be followed (with Follows).
 */
class IsFollowable$Type extends TraitClass<IsFollowable, TraitType.FOLLOWABLE> {}

export const IsFollowable = new IsFollowable$Type(TraitType.FOLLOWABLE);
registerTraitClass(TraitType.FOLLOWABLE, IsFollowable);
/* ==== DESTACK_GENERATED_END:TRAIT:1400034 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1800000 ==== */
/**
 * An Entity that can be presented visually.
 */
export interface IsViewable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be presented visually.
 */
class IsViewable$Type extends TraitClass<IsViewable, TraitType.VIEWABLE> {}

export const IsViewable = new IsViewable$Type(TraitType.VIEWABLE);
registerTraitClass(TraitType.VIEWABLE, IsViewable);
/* ==== DESTACK_GENERATED_END:TRAIT:1800000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:700003 ==== */
/**
 * An Entity that can be defined in a Script.
 */
export interface IsSourceable extends IsOrdered {
  get source(): Script | null;
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null;
  set key(value: string | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be defined in a Script.
 */
class IsSourceable$Type extends TraitClass<IsSourceable, TraitType.SOURCEABLE> {}

export const IsSourceable = new IsSourceable$Type(TraitType.SOURCEABLE);
registerTraitClass(TraitType.SOURCEABLE, IsSourceable);
/* ==== DESTACK_GENERATED_END:TRAIT:700003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:700002 ==== */
/**
 * An Entity that can be scripted.
 */
export interface IsScriptable {
  get script(): Script | null;
  set script(value: Script | null);
  /**
   * The main / root Script of this Node.
   */
  get scriptPtr(): NodeReference | null;
  set scriptPtr(value: NodeReference | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be scripted.
 */
class IsScriptable$Type extends TraitClass<IsScriptable, TraitType.SCRIPTABLE> {}

export const IsScriptable = new IsScriptable$Type(TraitType.SCRIPTABLE);
registerTraitClass(TraitType.SCRIPTABLE, IsScriptable);
/* ==== DESTACK_GENERATED_END:TRAIT:700002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:700001 ==== */
/**
 * An Entity that can be (directly, with Runs).
 */
export interface IsRunnable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be (directly, with Runs).
 */
class IsRunnable$Type extends TraitClass<IsRunnable, TraitType.RUNNABLE> {}

export const IsRunnable = new IsRunnable$Type(TraitType.RUNNABLE);
registerTraitClass(TraitType.RUNNABLE, IsRunnable);
/* ==== DESTACK_GENERATED_END:TRAIT:700001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10000 ==== */
/**
 * An Entity that can be ordered.
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
 * An Entity that can be ordered.
 */
class IsOrdered$Type extends TraitClass<IsOrdered, TraitType.ORDERED> {}

export const IsOrdered = new IsOrdered$Type(TraitType.ORDERED);
registerTraitClass(TraitType.ORDERED, IsOrdered);
/* ==== DESTACK_GENERATED_END:TRAIT:10000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10100 ==== */
/**
 * An Entity that can be customized with custom Properties.
 */
export interface IsCustomizable {
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  get customValues(): { readonly [key: string]: Value };
  set customValues(value: { readonly [key: string]: Value });

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be customized with custom Properties.
 */
class IsCustomizable$Type extends TraitClass<IsCustomizable, TraitType.CUSTOMIZABLE> {}

export const IsCustomizable = new IsCustomizable$Type(TraitType.CUSTOMIZABLE);
registerTraitClass(TraitType.CUSTOMIZABLE, IsCustomizable);
/* ==== DESTACK_GENERATED_END:TRAIT:10100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10200 ==== */
/**
 * A Node that be extended by custom Nodes (i.e. used as a base type).
 */
export interface IsExtensible extends IsCustomizable, IsScriptable {
  get definition(): Entity | null;
  readonly definitionPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

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
/* ==== DESTACK_GENERATED_END:TRAIT:10200 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10300 ==== */
/**
 * An Entity that is fixed / forward-only in spacetime.
 */
export interface IsIrreversible {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that is fixed / forward-only in spacetime.
 */
class IsIrreversible$Type extends TraitClass<IsIrreversible, TraitType.IRREVERSIBLE> {}

export const IsIrreversible = new IsIrreversible$Type(TraitType.IRREVERSIBLE);
registerTraitClass(TraitType.IRREVERSIBLE, IsIrreversible);
/* ==== DESTACK_GENERATED_END:TRAIT:10300 ==== */
