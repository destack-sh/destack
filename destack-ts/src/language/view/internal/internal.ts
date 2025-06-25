import { TraitClass, TraitType } from "@destack/language/core/builtin";
import { registerTraitClass } from "@destack/language/registry";
import { View } from "@destack/language/view";

/* ==== DESTACK_GENERATED_START:TRAIT:10650 ==== */
/**
 * A content View.
 */
export interface InternalView extends View {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A content View.
 */
class InternalView$Type extends TraitClass {}

export const InternalView = new InternalView$Type(TraitType.INTERNAL_VIEW);
registerTraitClass(TraitType.INTERNAL_VIEW, InternalView);
/* ==== DESTACK_GENERATED_END:TRAIT:10650 ==== */
