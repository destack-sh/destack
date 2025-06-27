import {
  Entity,
  Event,
  HasName,
  IsDeletable,
  IsOrdered,
  IsScriptable,
  IsTaggable,
  IsVisual,
  Spatial,
  TraitClass,
  TraitType,
} from "@destack/language/core/builtin";
import { Dimension, Position } from "@destack/language/core/common";
import { registerTraitClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:TRAIT:9001 ==== */
/**
 * A View is a graphical interface.
 */
export interface View
  extends Spatial,
    Entity,
    IsDeletable,
    IsOrdered,
    HasName,
    IsTaggable,
    IsScriptable,
    IsVisual {
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

/**
 * A View is a graphical interface.
 */
class View$Type extends TraitClass<View, TraitType.VIEW> {}

export const View = new View$Type(TraitType.VIEW);
registerTraitClass(TraitType.VIEW, View);
/* ==== DESTACK_GENERATED_END:TRAIT:9001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9002 ==== */
/**
 * A ViewEvent is an event on a View node.
 */
export interface ViewEvent extends Event {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

/**
 * A ViewEvent is an event on a View node.
 */
class ViewEvent$Type extends TraitClass<ViewEvent, TraitType.VIEW_EVENT> {}

export const ViewEvent = new ViewEvent$Type(TraitType.VIEW_EVENT);
registerTraitClass(TraitType.VIEW_EVENT, ViewEvent);
/* ==== DESTACK_GENERATED_END:TRAIT:9002 ==== */
