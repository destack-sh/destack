import type {
  IsDeletable,
  IsOrdered,
  IsSpatial,
  IsSubject,
  IsTaggable,
  Materialization,
  NodeReference,
  Snapshot,
} from "@destack/language/core";
import { Entity, Node, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { Palette } from "@destack/language/style/palette";
import type { Theme } from "@destack/language/style/theme";
import type { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:270200 ==== */
/**
 * A Style is a style definition.
 */
export abstract class Style
  extends Entity
  implements IsSpatial, IsOrdered, IsTaggable, IsDeletable
{
  static metatype: NodeType = NodeType.STYLE;

  abstract get parent(): Scene | View | Theme | Palette | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  abstract get predecessor(): Style | null;
  declare readonly predecessorPtr: NodeReference | null;

  abstract get template(): Style | null;
  declare readonly templatePtr: NodeReference | null;

  abstract get instanceRoot(): Entity | null;
  declare readonly instanceRootPtr: NodeReference | null;

  /**
   * Entity.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  abstract get createdBy(): (Node & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * Entity.updatedAt
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  abstract get updatedBy(): (Node & IsSubject) | null;
  declare readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The absolute order key of this Node in its parent.
   */
  declare readonly orderKey: string;

  /**
   * Style.name
   */
  /**
   * Style.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.STYLE, Style);
/* ==== DESTACK_GENERATED_END:NODE:270200 ==== */
