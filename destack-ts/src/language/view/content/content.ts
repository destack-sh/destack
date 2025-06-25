import { TraitClass, TraitType } from "@destack/language/core/builtin";
import { Align } from "@destack/language/core/common";
import { registerTraitClass } from "@destack/language/registry";
import { View } from "@destack/language/view";

/* ==== DESTACK_GENERATED_START:TRAIT:10200 ==== */
/**
 * A content View.
 */
export interface ContentView extends View {
  /**
   * ContentView.align
   */
  align: Align | null;

  /**
   * ContentView.isVisible
   */
  isVisible: boolean | null;

  /**
   * ContentView.opacity
   */
  opacity: number | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A content View.
 */
class ContentView$Type extends TraitClass {}

export const ContentView = new ContentView$Type(TraitType.CONTENT_VIEW);
registerTraitClass(TraitType.CONTENT_VIEW, ContentView);
/* ==== DESTACK_GENERATED_END:TRAIT:10200 ==== */
