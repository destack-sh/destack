import {
  Dimension,
  Entity,
  HasName,
  IsDeletable,
  IsOrdered,
  IsScriptable,
  IsTaggable,
  IsVisual,
  Position,
  Spatial,
} from "@destack/language/core";
import { TraitType } from "@destack/language/core/builtin";

/* ==== DESTACK_GENERATED_START:TRAIT:9001 ==== */
/**
 * A View is a graphical interface.
 */
export interface View extends Spatial, Entity, IsDeletable, IsOrdered, HasName, IsTaggable, IsScriptable, IsVisual {
  /**
   * View.position
   */
  position: Position | null;

  /**
   * View.width
   */
  width: Dimension | null;

  /**
   * View.height
   */
  height: Dimension | null;

  /**
   * View.minWidth
   */
  minWidth: Dimension | null;

  /**
   * View.minHeight
   */
  minHeight: Dimension | null;

  /**
   * View.maxWidth
   */
  maxWidth: Dimension | null;

  /**
   * View.maxHeight
   */
  maxHeight: Dimension | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class View$Type extends TraitFacade {}
export const View = new View$Type(TraitType.VIEW);
/* ==== DESTACK_GENERATED_END:TRAIT:9001 ==== */
