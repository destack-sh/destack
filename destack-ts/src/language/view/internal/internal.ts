import { TraitType } from "@destack/language/core/builtin";
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

class InternalView$Type extends TraitFacade {}
export const InternalView = new InternalView$Type(TraitType.INTERNAL_VIEW);
/* ==== DESTACK_GENERATED_END:TRAIT:10650 ==== */
