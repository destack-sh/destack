import { TraitClass, TraitType } from "@destack/language/core/builtin";
import { registerTraitClass } from "@destack/language/registry";
import { View } from "@destack/language/view";

/* ==== DESTACK_GENERATED_START:TRAIT:10600 ==== */
/**
 * A node View.
 */
export interface NodeView extends View {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A node View.
 */
class NodeView$Type extends TraitClass {}

export const NodeView = new NodeView$Type(TraitType.NODE_VIEW);
registerTraitClass(TraitType.NODE_VIEW, NodeView);
/* ==== DESTACK_GENERATED_END:TRAIT:10600 ==== */
