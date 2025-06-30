import {
  Entity,
  HasName,
  IsDeletable,
  IsOrdered,
  IsSpatial,
  IsSubject,
  IsTaggable,
  Node,
  NodeReference,
  NodeType,
} from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:12040 ==== */
/**
 * A Style is a style definition.
 */
export abstract class Style
  extends Entity
  implements IsSpatial, HasName, IsOrdered, IsTaggable, IsDeletable
{
  static metatype: NodeType = NodeType.STYLE;

  /**
   * Style.parent
   */
  get parent(): Scene | View | Theme | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | View | Theme | null;
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.STYLE, Style);
/* ==== DESTACK_GENERATED_END:NODE:12040 ==== */
