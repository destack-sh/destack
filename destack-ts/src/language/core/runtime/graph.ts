import type { Entity, Node, NodeClass } from "@destack/language/core/builtin";
import { NodeType } from "@destack/language/core/builtin/common";
import { TraitClass } from "@destack/language/core/builtin/trait";
import { NODE_CLASS_BY_TYPE, NODE_TYPES_BY_TRAIT_TYPE } from "@destack/language/registry";

/** A Graph is a collection of Entity. */
export abstract class Graph {
  /** Get a string representation of the Graph. */
  repr(): string {
    return `<${this.constructor.name}>`;
  }

  /** Get an Entity by id. */
  abstract get(id: string): Entity | null;

  /** Get an Entity by id, or throw an error if not found. */
  getOrError(id: string): Entity {
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

  /** Add an Entity to the Graph (must not exist, excluding descendants). */
  abstract add(node: Entity): void;

  /** Remove an Entity from the Graph (must exist, excluding descendants). */
  abstract remove(node: Entity): void;

  /**
   * Collect child Entities (one level down).
   * If the Entities are IsOrdered, their order is preserved.
   */
  abstract getChildren(options: { node: Entity; nodeType?: NodeType }): Entity[];

  /**
   * Collect descendant Entities (recursively down).
   * If a type is specified, only Entities of that type are collected.
   * (Descendants are not collected unless all their ancestors are included).
   * Entities are BFS but IsOrdered is ignored.
   */
  abstract getDescendants(options: { node: Entity; nodeType?: NodeType }): Entity[];
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
