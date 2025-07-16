import type {
  IsActor,
  IsDeletable,
  IsExtensible,
  IsOrdered,
  IsTaggable,
  Materialization,
  NodeReference,
  Snapshot,
  Value,
} from "@destack/language/core";
import { Entity, NodeType } from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Palette } from "@destack/language/style/palette";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:2100200 ==== */
/**
 * A Style defines a base visual appearance in some context.
 */
export abstract class Style
  extends Entity
  implements IsOrdered, IsTaggable, IsDeletable, IsExtensible
{
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
   * The definition this CustomEntity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionPtr: NodeReference | null;

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
  abstract get precededBy(): Style | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): (Entity & IsActor) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): (Entity & IsActor) | null;
  declare readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  abstract get customValues(): { readonly [key: string]: Value };
  abstract set customValues(value: { readonly [key: string]: Value });

  /**
   * The absolute order key of this Node in its parent.
   */
  declare readonly orderKey: string;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /**
   * The main / root Script of this Node.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The main / root Script of this Node.
   */
  abstract get scriptPtr(): NodeReference | null;
  abstract set scriptPtr(value: NodeReference | null);

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  declare readonly isExtensible: boolean;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.STYLE, Style);
/* ==== DESTACK_GENERATED_END:NODE:2100200 ==== */
