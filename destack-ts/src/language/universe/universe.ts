import type { NodeReference } from "@destack/language/core";
import { Node, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:NODE:20000 ==== */
/**
 * The Universe of Destack.
 * An abstract container for useful constants.
 */
export abstract class Universe extends Node {
  static metatype: NodeType = NodeType.UNIVERSE;

  abstract get parent(): Node | null;
  declare readonly parentPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.UNIVERSE, Universe);
/* ==== DESTACK_GENERATED_END:NODE:20000 ==== */
