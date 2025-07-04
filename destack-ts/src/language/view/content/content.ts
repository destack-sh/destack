import type {
  Align,
  CustomEntityDefinition,
  CustomEventDefinition,
  Dimension,
  IsSubject,
  Materialization,
  NodeDefinitionReference,
  NodeReference,
  Position,
  Snapshot,
  Value,
} from "@destack/language/core";
import { Entity, Node, NodeType } from "@destack/language/core";
import type { Folder } from "@destack/language/folder";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Layer, Scene, Window } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { ContainerView } from "@destack/language/view/container/container";
import { View } from "@destack/language/view/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:220000 ==== */
/**
 * A content View.
 */
export abstract class ContentView extends View {
  static metatype: NodeType = NodeType.CONTENT_VIEW;

  abstract get parent(): Window | Scene | Layer | ContainerView | Folder | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  abstract get definition(): CustomEntityDefinition | CustomEventDefinition | null;
  declare readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  declare readonly baseType: NodeDefinitionReference | null;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  abstract get predecessor(): ContentView | null;
  declare readonly predecessorPtr: NodeReference | null;

  abstract get template(): ContentView | null;
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  abstract get customValues(): Map<string, Value>;
  abstract set customValues(value: Map<string, Value>);

  /**
   * The absolute order key of this Node in its parent.
   */
  declare readonly orderKey: string;

  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The main / root Script of this Node.
   */
  abstract get scriptPtr(): NodeReference | null;
  abstract set scriptPtr(value: NodeReference | null);

  /**
   * View.name
   */
  /**
   * View.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /**
   * View.position
   */
  /**
   * View.position
   */
  abstract get position(): Position | null;
  abstract set position(value: Position | null);

  /**
   * View.width
   */
  /**
   * View.width
   */
  abstract get width(): Dimension | null;
  abstract set width(value: Dimension | null);

  /**
   * View.height
   */
  /**
   * View.height
   */
  abstract get height(): Dimension | null;
  abstract set height(value: Dimension | null);

  /**
   * View.minWidth
   */
  /**
   * View.minWidth
   */
  abstract get minWidth(): Dimension | null;
  abstract set minWidth(value: Dimension | null);

  /**
   * View.minHeight
   */
  /**
   * View.minHeight
   */
  abstract get minHeight(): Dimension | null;
  abstract set minHeight(value: Dimension | null);

  /**
   * View.maxWidth
   */
  /**
   * View.maxWidth
   */
  abstract get maxWidth(): Dimension | null;
  abstract set maxWidth(value: Dimension | null);

  /**
   * View.maxHeight
   */
  /**
   * View.maxHeight
   */
  abstract get maxHeight(): Dimension | null;
  abstract set maxHeight(value: Dimension | null);

  /**
   * ContentView.align
   */
  /**
   * ContentView.align
   */
  abstract get align(): Align | null;
  abstract set align(value: Align | null);

  /**
   * ContentView.isVisible
   */
  /**
   * ContentView.isVisible
   */
  abstract get isVisible(): boolean | null;
  abstract set isVisible(value: boolean | null);

  /**
   * ContentView.opacity
   */
  /**
   * ContentView.opacity
   */
  abstract get opacity(): number | null;
  abstract set opacity(value: number | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CONTENT_VIEW, ContentView);
/* ==== DESTACK_GENERATED_END:NODE:220000 ==== */
