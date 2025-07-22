import type { Entity } from "@destack/language/core/builtin";
import { NodeType, TraitType } from "@destack/language/core/builtin/common";
import { expandNodeTypes, Graph } from "@destack/language/core/runtime/graph";
import { NODE_CLASS_BY_TYPE } from "@destack/language/registry";
import { INTEGER_ZERO } from "@destack/utils/fractional";

/** A Graph that stores Entities in memory. */
export class MemoryGraph extends Graph {
  private nodesById: Map<string, Entity>;
  private nodesByParent: Map<string, Map<NodeType, Entity[]>>;

  constructor() {
    super();
    this.nodesById = new Map();
    this.nodesByParent = new Map();
  }

  override get(id: string): Entity | null {
    return this.nodesById.get(id) ?? null;
  }

  override has(id: string): boolean {
    return this.nodesById.has(id);
  }

  override clear(): void {
    // nodes
    this.nodesById.clear();
    this.nodesByParent.clear();
  }

  override add(node: Entity): void {
    const existing = this.nodesById.get(node.id);
    if (existing !== undefined) {
      throw new Error(`node ${node.repr()} already in ${this.repr()}: ${existing.repr()}`);
    }
    // node
    this.nodesById.set(node.id, node);
    // parent
    const parentPtr = (node as any).parentPtr;
    if (parentPtr !== null && parentPtr !== undefined) {
      if (!this.nodesByParent.has(parentPtr.id)) {
        this.nodesByParent.set(parentPtr.id, new Map());
      }
      const childNodeType = node.metatype;
      const parentMap = this.nodesByParent.get(parentPtr.id)!;
      if (!parentMap.has(childNodeType)) {
        parentMap.set(childNodeType, []);
      }
      parentMap.get(childNodeType)!.push(node);
    }
  }

  override remove(node: Entity): void {
    // parent
    const parentPtr = (node as any).parentPtr;
    if (parentPtr !== null && parentPtr !== undefined) {
      const parentMap = this.nodesByParent.get(parentPtr.id);
      if (parentMap) {
        const childNodeType = node.metatype;
        const children = parentMap.get(childNodeType);
        if (children) {
          const index = children.indexOf(node);
          if (index !== -1) {
            children.splice(index, 1);
          }
          if (children.length === 0) {
            parentMap.delete(childNodeType);
            if (parentMap.size === 0) {
              this.nodesByParent.delete(parentPtr.id);
            }
          }
        }
      }
    }
    // node
    this.nodesById.delete(node.id);
  }

  override getChildren(options: { node: Entity; nodeType?: NodeType }): Entity[] {
    // bail if no children
    if (this.nodesByParent.size === 0) {
      return [];
    }
    const childrenByType = this.nodesByParent.get(options.node.id);
    if (!childrenByType) {
      return [];
    }

    if (options.nodeType === undefined) {
      // collect children across all types
      const children: Entity[] = [];
      let isOrdered = false;
      for (const childrenOfType of childrenByType.values()) {
        if (childrenOfType.length > 0) {
          const nodeClass = NODE_CLASS_BY_TYPE[childrenOfType[0].metatype];
          if (nodeClass.__definition__.traits.includes(TraitType.ORDERED)) {
            isOrdered = true;
          }
          children.push(...childrenOfType);
        }
      }
      if (isOrdered) {
        children.sort((a, b) => {
          const aOrderKey = (a as any).orderKey ?? INTEGER_ZERO;
          const bOrderKey = (b as any).orderKey ?? INTEGER_ZERO;
          return aOrderKey.localeCompare(bOrderKey);
        });
      }
      return children;
    } else {
      // turn into type
      const nodeTypes = expandNodeTypes(options.nodeType, { expandInheritance: true });
      if (!nodeTypes || nodeTypes.length === 0) {
        return [];
      }
      const nodeClass = NODE_CLASS_BY_TYPE[nodeTypes[0]];

      // collect
      if (nodeTypes.length === 1) {
        // collect for single node type
        const children = childrenByType.get(nodeTypes[0]) ?? [];
        if (children.length > 0 && nodeClass.__definition__.traits.includes(TraitType.ORDERED)) {
          const sortedChildren = [...children];
          sortedChildren.sort((a, b) => {
            const aOrderKey = (a as any).orderKey ?? INTEGER_ZERO;
            const bOrderKey = (b as any).orderKey ?? INTEGER_ZERO;
            return aOrderKey.localeCompare(bOrderKey);
          });
          return sortedChildren;
        }
        return children;
      } else {
        // collect for trait (multiple node types)
        const children: Entity[] = [];
        for (const nodeType of nodeTypes) {
          children.push(...(childrenByType.get(nodeType) ?? []));
        }
        if (children.length > 0 && nodeClass.__definition__.traits.includes(TraitType.ORDERED)) {
          children.sort((a, b) => {
            const aOrderKey = (a as any).orderKey ?? INTEGER_ZERO;
            const bOrderKey = (b as any).orderKey ?? INTEGER_ZERO;
            return aOrderKey.localeCompare(bOrderKey);
          });
        }
        return children;
      }
    }
  }

  override getDescendants(options: { node: Entity; nodeType?: NodeType }): Entity[] {
    if (this.nodesByParent.size === 0) {
      return [];
    }

    const queue: Entity[] = [options.node];
    const descendants: Entity[] = [];
    const nodeTypes = expandNodeTypes(options.nodeType, { expandInheritance: true });

    // BFS
    while (queue.length > 0) {
      const current = queue.shift()!;
      const childrenByType = this.nodesByParent.get(current.id);
      if (!childrenByType) {
        continue;
      }

      // add all children to queue
      for (const childrenOfType of childrenByType.values()) {
        queue.push(...childrenOfType);
      }

      // collect descendants based on type filter
      if (!nodeTypes) {
        for (const childrenOfType of childrenByType.values()) {
          descendants.push(...childrenOfType);
        }
      } else {
        for (const nodeType of nodeTypes) {
          descendants.push(...(childrenByType.get(nodeType) ?? []));
        }
      }
    }

    return descendants;
  }

  /**
   * Get ancestors of a node (going up the parent chain).
   * If a type is specified, only ancestors of that type are collected.
   */
  getAncestors(node: Entity, nodeType?: NodeType): Entity[] {
    const ancestors: Entity[] = [];
    let currentPtr = (node as any).parentPtr;
    const nodeTypes = expandNodeTypes(nodeType, { expandInheritance: true });

    // traverse up the parent chain
    while (currentPtr !== null && currentPtr !== undefined) {
      const parentNode = this.get(currentPtr.id);
      if (parentNode === null) {
        break;
      }
      if (nodeTypes === null || nodeTypes.includes(parentNode.metatype)) {
        ancestors.push(parentNode);
      }
      currentPtr = (parentNode as any).parentPtr;
    }

    return ancestors;
  }
}
