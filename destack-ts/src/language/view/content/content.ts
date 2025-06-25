import { Align } from "@destack/language/core";
import { TraitType } from "@destack/language/core/builtin";
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

class ContentView$Type extends TraitFacade {}
export const ContentView = new ContentView$Type(TraitType.CONTENT_VIEW);
/* ==== DESTACK_GENERATED_END:TRAIT:10200 ==== */
