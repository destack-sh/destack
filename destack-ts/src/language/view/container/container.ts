import {
  Align,
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Direction,
  Distribute,
  Grid,
  GridSpan,
  Insets,
  IsExtensible,
  IsSubject,
  Layout,
  Node,
  NodeReference,
  NodeType,
  Position,
  Value,
  Vector2,
} from "@destack/language/core";
import { Folder } from "@destack/language/folder";
import { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import { Layer, Scene, Window } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Border, Fill, Shadow } from "@destack/language/style";
import { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:10000 ==== */
/**
 * A container View contains other Views.
 */
export abstract class ContainerView extends View implements IsExtensible {
  static metatype: NodeType = NodeType.CONTAINER_VIEW;

  /**
   * View.parent
   */
  get parent(): Window | Scene | Layer | ContainerView | Folder | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | Window
        | Scene
        | Layer
        | ContainerView
        | Folder
        | null;
    }
    return null;
  }
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  declare readonly spacePtr: NodeReference | null;

  /**
   * Entity.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  declare readonly createdByPtr: NodeReference | null;

  /**
   * Entity.updatedAt
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  declare readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsExtensible.value
   */
  declare value: Map<string, Value>;

  /**
   * IsOrdered.orderKey
   */
  declare readonly orderKey: string;

  /**
   * HasName.name
   */
  declare name: string;

  /**
   * View.position
   */
  declare position: Position | null;

  /**
   * View.width
   */
  declare width: Dimension | null;

  /**
   * View.height
   */
  declare height: Dimension | null;

  /**
   * View.minWidth
   */
  declare minWidth: Dimension | null;

  /**
   * View.minHeight
   */
  declare minHeight: Dimension | null;

  /**
   * View.maxWidth
   */
  declare maxWidth: Dimension | null;

  /**
   * View.maxHeight
   */
  declare maxHeight: Dimension | null;

  /**
   * ContainerView.layout
   */
  declare layout: Layout | null;

  /**
   * ContainerView.direction
   */
  declare direction: Direction | null;

  /**
   * ContainerView.distribute
   */
  declare distribute: Distribute | null;

  /**
   * ContainerView.align
   */
  declare align: Align | null;

  /**
   * ContainerView.gap
   */
  declare gap: Axis2 | null;

  /**
   * ContainerView.padding
   */
  declare padding: Insets | null;

  /**
   * ContainerView.grid
   */
  declare grid: Grid | null;

  /**
   * ContainerView.gridSpan
   */
  declare gridSpan: GridSpan | null;

  /**
   * ContainerView.aspectRatio
   */
  declare aspectRatio: number | null;

  /**
   * ContainerView.isWrap
   */
  declare isWrap: boolean | null;

  /**
   * ContainerView.isVisible
   */
  declare isVisible: boolean | null;

  /**
   * ContainerView.opacity
   */
  declare opacity: number | null;

  /**
   * ContainerView.fill
   */
  declare fill: Fill | null;

  /**
   * ContainerView.rotation
   */
  declare rotation: Axis3 | null;

  /**
   * ContainerView.skew
   */
  declare skew: Vector2 | null;

  /**
   * ContainerView.scale
   */
  declare scale: number | null;

  /**
   * ContainerView.shadow
   */
  declare shadow: Shadow | null;

  /**
   * ContainerView.border
   */
  declare border: Border | null;

  /**
   * ContainerView.radius
   */
  declare radius: Corners | null;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptPtr = null;
    } else {
      this.scriptPtr = node.toRef();
    }
  }
  declare scriptPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CONTAINER_VIEW, ContainerView);
/* ==== DESTACK_GENERATED_END:NODE:10000 ==== */
