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
/* ==== DESTACK_GENERATED_END:TRAIT:9001 ==== */
