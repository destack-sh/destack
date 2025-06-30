import {
  Dimension,
  Entity,
  HasName,
  IsDeletable,
  IsOrdered,
  IsScriptable,
  IsSpatial,
  IsSubject,
  IsTaggable,
  Node,
  NodeReference,
  NodeType,
  Position,
} from "@destack/language/core";
import { Folder } from "@destack/language/folder";
import { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import { Layer, Scene, Window } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { ContainerView } from "@destack/language/view/container";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:9100 ==== */
/**
 * A View is a graphical interface.
 */
export abstract class View
  extends Entity
  implements IsSpatial, HasName, IsOrdered, IsTaggable, IsScriptable, IsDeletable
{
  static metatype: NodeType = NodeType.VIEW;

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
registerNodeClass(NodeType.VIEW, View);
/* ==== DESTACK_GENERATED_END:NODE:9100 ==== */
