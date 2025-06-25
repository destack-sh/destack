import {
  Entity,
  HasName,
  IsDeletable,
  IsOrdered,
  IsTaggable,
  IsVisual,
  Spatial,
  TraitClass,
  TraitType,
} from "@destack/language/core/builtin";
import { registerTraitClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:TRAIT:12000 ==== */
/**
 * A Style is a style definition.
 */
export interface Style extends Spatial, Entity, IsDeletable, IsOrdered, HasName, IsTaggable, IsVisual {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A Style is a style definition.
 */
class Style$Type extends TraitClass {}

export const Style = new Style$Type(TraitType.STYLE);
registerTraitClass(TraitType.STYLE, Style);
/* ==== DESTACK_GENERATED_END:TRAIT:12000 ==== */
