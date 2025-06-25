import { TraitClass, TraitType } from "@destack/language/core/builtin";
import { registerTraitClass } from "@destack/language/registry";
import { View } from "@destack/language/view";

/* ==== DESTACK_GENERATED_START:TRAIT:10400 ==== */
/**
 * An input View.
 */
export interface InputView extends View {
  /**
   * InputView.isVisible
   */
  isVisible: boolean | null;

  /**
   * InputView.opacity
   */
  opacity: number | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * An input View.
 */
class InputView$Type extends TraitClass {}

export const InputView = new InputView$Type(TraitType.INPUT_VIEW);
registerTraitClass(TraitType.INPUT_VIEW, InputView);
/* ==== DESTACK_GENERATED_END:TRAIT:10400 ==== */
