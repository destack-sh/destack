import type {
  IsDeletable,
  IsOrdered,
  IsSubject,
  IsTaggable,
  Materialization,
  NodeReference,
  Snapshot,
} from "@destack/language/core";
import { Entity, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Palette } from "@destack/language/style/palette";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:600200 ==== */
/**
 * A Style defines a base visual appearance in some context.
 */
export abstract class Style extends Entity implements IsOrdered, IsTaggable, IsDeletable {
  static metatype: NodeType = NodeType.STYLE;

  /**
   * Style.parent
   */
  abstract get parent(): Scene | View | Theme | Palette | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  abstract get predecessor(): Style | null;
  declare readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  abstract get template(): Style | null;
  declare readonly templatePtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Subject that created this Entity.
   */
  abstract get createdBy(): (Entity & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Subject that last updated this Entity.
   */
  abstract get updatedBy(): (Entity & IsSubject) | null;
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
/* ==== DESTACK_GENERATED_END:NODE:600200 ==== */
