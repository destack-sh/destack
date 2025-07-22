import type { Node, NodeClass } from "@destack/language/core/builtin";
import { NodeType } from "@destack/language/core/builtin/common";
import { TraitClass } from "@destack/language/core/builtin/trait";
import { NODE_CLASS_BY_TYPE, NODE_TYPES_BY_TRAIT_TYPE } from "@destack/language/registry";

/** A Graph is a collection of Nodes. */
export abstract class Graph<TNode extends Node = Node> {
  repr(): string {
    return `<${this.constructor.name} ${this.nodes.length} nodes>`;
  }

  /** Get the number of Nodes in the Graph. */
  abstract get size(): number;

  /** Get all Nodes in the Graph. */
  abstract get nodes(): TNode[];

  /** Get a Node by id. */
  abstract get(id: string): TNode | null;

  /** Get a Node by id, or throw an error if not found. */
  getOrError(id: string): TNode {
    const node = this.get(id);
    if (!node) {
      throw new Error(`Node ${id} not found in ${this.constructor.name}`);
    }
    return node;
  }

  /** Check if a Node exists in this Graph. */
  abstract has(id: string): boolean;

  /** Clear the Graph. */
  abstract clear(): void;

  /** Add a Node to the Graph (must not exist, excluding descendants). */
  abstract add(node: TNode): void;

  /** Remove a Node from the Graph (must exist, excluding descendants). */
  abstract remove(node: TNode): void;

  /**
   * Collect child Nodes (one level down).
   * If the Nodes are IsOrdered, their order is preserved.
   */
  abstract getChildren(options: { node: TNode; nodeType?: NodeType }): TNode[];

  /**
   * Collect descendant Nodes (recursively down).
   * If a type is specified, only Nodes of that type are collected.
   * (Descendants are not collected unless all their ancestors are included).
   * Nodes are BFS but IsOrdered is ignored.
   */
  abstract getDescendants(options: { node: TNode; nodeType?: NodeType }): TNode[];
}

/** Expand a collection of NodeTypes into a flat collection of NodeTypes. */
export function expandNodeInheritance(nodeTypes: NodeType[]): NodeType[] {
  const expanded: NodeType[] = [];
  for (const type of nodeTypes) {
    const nodeDefinition = NODE_CLASS_BY_TYPE[type].__definition__;
    for (const inheritedType of nodeDefinition.inheritedBy) {
      if (!expanded.includes(inheritedType)) {
        expanded.push(inheritedType);
      }
    }
    if (!nodeDefinition.isAbstract && !expanded.includes(type)) {
      expanded.push(type);
    }
  }
  return expanded;
}

/** Resolve the NodeTypes for a NodeType, TraitType, or Node class. */
export function expandNodeTypes(
  nodeType?: NodeType | NodeClass | TraitClass,
  options: { expandInheritance: boolean } = { expandInheritance: true },
): NodeType[] | null {
  if (nodeType == null) {
    return null;
  }
  let nodeTypes: NodeType[] = [];
  if (nodeType instanceof TraitClass) {
    const traitType = nodeType.metatype;
    nodeTypes.push(...(NODE_TYPES_BY_TRAIT_TYPE[traitType] ?? []));
  } else if (typeof nodeType == "number") {
    nodeTypes.push(nodeType);
  } else {
    nodeTypes.push(nodeType.metatype);
  }

  if (options.expandInheritance) {
    nodeTypes = expandNodeInheritance(nodeTypes);
  }

  return nodeTypes;
}
