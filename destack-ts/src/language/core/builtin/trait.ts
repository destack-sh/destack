import { TraitType } from "@destack/language/core/builtin/common";
import type {
  PropertyDefinition,
  TraitDefinition,
} from "@destack/language/core/builtin/definition";
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

/* ==== DESTACK_GENERATED_START:TRAIT:2000000 ==== */
/**
 * An Entity that can be interacted with.
 */
export interface IsInteractive {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be interacted with.
 */
class IsInteractive$Type extends TraitClass<IsInteractive, TraitType.INTERACTIVE> {}

export const IsInteractive = new IsInteractive$Type(TraitType.INTERACTIVE);
registerTraitClass(TraitType.INTERACTIVE, IsInteractive);
/* ==== DESTACK_GENERATED_END:TRAIT:2000000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2000001 ==== */
/**
 * An Entity that can be dragged.
 */
export interface IsDraggable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be dragged.
 */
class IsDraggable$Type extends TraitClass<IsDraggable, TraitType.DRAGGABLE> {}

export const IsDraggable = new IsDraggable$Type(TraitType.DRAGGABLE);
registerTraitClass(TraitType.DRAGGABLE, IsDraggable);
/* ==== DESTACK_GENERATED_END:TRAIT:2000001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2000002 ==== */
/**
 * An Entity that can be selected.
 */
export interface IsSelectable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An Entity that can be selected.
 */
class IsSelectable$Type extends TraitClass<IsSelectable, TraitType.SELECTABLE> {}

export const IsSelectable = new IsSelectable$Type(TraitType.SELECTABLE);
registerTraitClass(TraitType.SELECTABLE, IsSelectable);
/* ==== DESTACK_GENERATED_END:TRAIT:2000002 ==== */
