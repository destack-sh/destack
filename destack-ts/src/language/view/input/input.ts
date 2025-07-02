import type {
  CustomEntityDefinition,
  CustomEventDefinition,
  Dimension,
  IsSubject,
  NodeDefinitionReference,
  NodeReference,
  Position,
  Value,
} from "@destack/language/core";
import { Node, NodeType } from "@destack/language/core";
import type { Folder } from "@destack/language/folder";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Layer, Scene, Window } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { ContainerView } from "@destack/language/view/container/container";
import { View } from "@destack/language/view/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:230000 ==== */
/**
 * An input View.
 */
export abstract class InputView extends View {
  static metatype: NodeType = NodeType.INPUT_VIEW;

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
   * The definitionthis CustomEntity is an instance of.
   */
  get definition(): CustomEntityDefinition | CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | null;
    }
    return null;
  }
  declare readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  declare readonly baseType: NodeDefinitionReference | null;

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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  declare customValues: Map<string, Value>;

  /**
   * The absolute order key of this Node in its parent.
   */
  declare readonly orderKey: string;

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

  /**
   * View.name
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
   * InputView.isVisible
   */
  declare isVisible: boolean | null;

  /**
   * InputView.opacity
   */
  declare opacity: number | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INPUT_VIEW, InputView);
/* ==== DESTACK_GENERATED_END:NODE:230000 ==== */
