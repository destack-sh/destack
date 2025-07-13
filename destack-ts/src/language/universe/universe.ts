import type { NodeReference } from "@destack/language/core";
import { Entity, Node, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Space } from "@destack/language/universe/space";

/* ==== DESTACK_GENERATED_START:NODE:20000 ==== */
/**
 * The Universe of Destack.
 * An abstract container for useful constants.
 */
export abstract class Universe extends Node {
  static metatype: NodeType = NodeType.UNIVERSE;

  /**
   * Node.parent
   */
  abstract get parent(): Entity | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.UNIVERSE, Universe);
/* ==== DESTACK_GENERATED_END:NODE:20000 ==== */
