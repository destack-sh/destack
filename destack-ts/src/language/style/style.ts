import { Entity, HasName, IsDeletable, IsOrdered, IsTaggable, IsVisual, Spatial } from "@destack/language/core";
import { TraitType } from "@destack/language/core/builtin";

/* ==== DESTACK_GENERATED_START:TRAIT:12000 ==== */
/**
 * A Style is a style definition.
 */
export interface Style extends Spatial, Entity, IsDeletable, IsOrdered, HasName, IsTaggable, IsVisual {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Style$Type extends TraitFacade {}
export const Style = new Style$Type(TraitType.STYLE);
/* ==== DESTACK_GENERATED_END:TRAIT:12000 ==== */
