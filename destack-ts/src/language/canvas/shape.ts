import { TraitClass, TraitType } from "@destack/language/core/builtin";
import { registerTraitClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:TRAIT:11000 ==== */
/**
 * A Node that is a Shape.
 */
export interface IsShape {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Node that is a Shape.
 */
class IsShape$Type extends TraitClass {}

export const IsShape = new IsShape$Type(TraitType.SHAPE);
registerTraitClass(TraitType.SHAPE, IsShape);
/* ==== DESTACK_GENERATED_END:TRAIT:11000 ==== */
