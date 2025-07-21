import type { Entity, Event, Node, NodeClass } from "@destack/language/core/builtin";
import { NodeType, TraitType } from "@destack/language/core/builtin/common";
import { hasTrait } from "@destack/language/core/builtin/node";
import { TraitClass } from "@destack/language/core/builtin/trait";
import type { Session } from "@destack/language/core/runtime/session";
import { NODE_CLASS_BY_TYPE, NODE_TYPES_BY_TRAIT_TYPE } from "@destack/language/registry";
import { INTEGER_ZERO } from "@destack/utils/fractional";

/** A Graph is a collection of Nodes. */
export abstract class Graph<TNode extends Node = Node> {
  readonly supergraph: Supergraph;

  constructor(supergraph: Supergraph) {
    this.supergraph = supergraph;
  }

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

  /** Find root Nodes in the graph. */
  abstract getRoots(options?: { nodeType?: NodeType }): TNode[];

  /** Find leaf Nodes in the graph. */
  abstract getLeaves(options?: { nodeType?: NodeType; node?: TNode }): TNode[];

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

/** A Graph that contains only a single Node. */
export class EntitySingletonGraph extends Graph<Entity> {
  readonly node: Entity;

  constructor(supergraph: Supergraph, node: Entity) {
    super(supergraph);
    this.node = node;
  }

  override get size(): number {
    return 1;
  }

  override get nodes(): Entity[] {
    return [this.node];
  }

  override get(id: string): Entity | null {
    return this.node.id === id ? this.node : null;
  }

  override has(id: string): boolean {
    return this.node.id === id;
  }

  override clear(): void {
    throw new Error("cannot clear a SingletonGraph");
  }

  override add(node: Entity): void {
    throw new Error("cannot add a Node to a SingletonGraph");
  }

  override remove(node: Entity): void {
    throw new Error("cannot remove a Node from a SingletonGraph");
  }

  override getRoots(): Entity[] {
    return [this.node];
  }

  override getLeaves(): Entity[] {
    return [this.node];
  }

  override getChildren(options: { node: Entity; nodeType?: NodeType }): Entity[] {
    return [];
  }

  override getDescendants(options: { node: Entity; nodeType?: NodeType }): Entity[] {
    return [];
  }
}

/** A Graph with an arbitrary, hierarchical set of Entities. */
export class EntityGraph extends Graph<Entity> {
  readonly nodesById: Map<string, Entity>;
  readonly nodesByParent: Map<string, Map<NodeType, Entity[]>>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this.nodesById = new Map();
    this.nodesByParent = new Map();
  }

  override get size(): number {
    return this.nodes.length;
  }

  override get nodes(): Entity[] {
    return Array.from(this.nodesById.values());
  }

  override get(id: string): Entity | null {
    return this.nodesById.get(id) ?? null;
  }

  override has(id: string): boolean {
    return this.nodesById.has(id);
  }

  override clear(): void {
    // supergraph
    for (const node of this.nodes) {
      if (this.supergraph._cachedNodesById.get(node.id) === node) {
        this.supergraph._cachedNodesById.delete(node.id);
      }
    }
    // nodes
    this.nodesById.clear();
    this.nodesByParent.clear();
  }

  override add(node: Entity): void {
    const existing = this.nodesById.get(node.id);
    if (existing !== undefined) {
      throw new Error(`node ${node} already in ${this}: ${existing}`);
    }
    // node
    this.nodesById.set(node.id, node);

    // parent
    if ((node as any).parentPtr !== null) {
      if (!this.nodesByParent.has((node as any).parentPtr.id)) {
        this.nodesByParent.set((node as any).parentPtr.id, new Map());
      }
      const childNodeType = node.metatype;
      const parentMap = this.nodesByParent.get((node as any).parentPtr.id)!;
      if (!parentMap.has(childNodeType)) {
        parentMap.set(childNodeType, []);
      }
      parentMap.get(childNodeType)!.push(node);
    }

    // supergraph
    const cached = this.supergraph._cachedNodesById.get(node.id);
    if (cached === undefined || cached === _MISSING) {
      this.supergraph._cachedNodesById.set(node.id, node);
    }
  }

  override remove(node: Entity): void {
    // supergraph
    if (this.supergraph._cachedNodesById.get(node.id) === node) {
      this.supergraph._cachedNodesById.delete(node.id);
    }

    // parent
    if ((node as any).parentPtr !== null) {
      const parentMap = this.nodesByParent.get((node as any).parentPtr.id);
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
              this.nodesByParent.delete((node as any).parentPtr.id);
            }
          }
        }
      }
    }

    // node
    this.nodesById.delete(node.id);
  }

  override getRoots(options?: { nodeType?: NodeType }): Entity[] {
    const nodeTypes = expandNodeTypes(options?.nodeType, { expandInheritance: true });
    if (nodeTypes === null) {
      return this.nodes.filter((node) => (node as any).parentPtr === null);
    }
    return this.nodes.filter(
      (node) => (node as any).parentPtr === null && nodeTypes.includes(node.metatype),
    );
  }

  override getLeaves(options?: { nodeType?: NodeType; node?: Entity }): Entity[] {
    const nodeTypes = expandNodeTypes(options?.nodeType, { expandInheritance: true });
    if (options?.node == null) {
      if (nodeTypes === null) {
        return this.nodes.filter((node) => !this.nodesByParent.has(node.id));
      } else {
        return this.nodes.filter(
          (node) => !this.nodesByParent.has(node.id) && nodeTypes.includes(node.metatype),
        );
      }
    } else {
      const descendants = this.getDescendants({ node: options.node });
      if (nodeTypes === null) {
        return descendants.filter((node) => !this.nodesByParent.has(node.id));
      } else {
        return descendants.filter(
          (node) => !this.nodesByParent.has(node.id) && nodeTypes.includes(node.metatype),
        );
      }
    }
  }

  override getChildren(options: { node: Entity; nodeType?: NodeType | NodeType[] }): Entity[] {
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
        const nodeClass = childrenOfType[0].constructor as NodeClass;
        if (nodeClass.__definition__.traits.includes(TraitType.ORDERED)) {
          isOrdered = true;
        }
        children.push(...childrenOfType);
      }
      if (isOrdered) {
        children.sort((a, b) => {
          const aOrderKey = (a as any).orderKey || 0;
          const bOrderKey = (b as any).orderKey || 0;
          return aOrderKey - bOrderKey;
        });
      }
      return children;
    } else {
      // turn into type
      const nodeTypes = Array.isArray(options.nodeType)
        ? options.nodeType
        : expandNodeTypes(options.nodeType, { expandInheritance: true });
      if (nodeTypes === null) {
        // collect children across all types
        const children: Entity[] = [];
        let isOrdered = false;
        for (const childrenOfType of childrenByType.values()) {
          const nodeClass = childrenOfType[0].constructor as NodeClass;
          if (nodeClass.__definition__.traits.includes(TraitType.ORDERED)) {
            isOrdered = true;
          }
          children.push(...childrenOfType);
        }
        if (isOrdered) {
          children.sort((a, b) => {
            const aOrderKey = (a as any).orderKey || 0;
            const bOrderKey = (b as any).orderKey || 0;
            return aOrderKey - bOrderKey;
          });
        }
        return children;
      } else {
        if (nodeTypes.length === 1) {
          // collect for single node type
          const children = childrenByType.get(nodeTypes[0]) || [];
          if (children.length > 0) {
            if (hasTrait(children[0], TraitType.ORDERED)) {
              children.sort((a, b) => {
                const aOrderKey = (a as Entity).orderKey || INTEGER_ZERO;
                const bOrderKey = (b as Entity).orderKey || INTEGER_ZERO;
                return aOrderKey.localeCompare(bOrderKey);
              });
            }
          }
          return children;
        } else {
          // collect for trait (multiple node types)
          const children: Entity[] = [];
          for (const nodeType of nodeTypes) {
            children.push(...(childrenByType.get(nodeType) || []));
          }
          if (children.length > 0) {
            if (hasTrait(children[0], TraitType.ORDERED)) {
              children.sort((a, b) => {
                const aOrderKey = (a as Entity).orderKey || INTEGER_ZERO;
                const bOrderKey = (b as Entity).orderKey || INTEGER_ZERO;
                return aOrderKey.localeCompare(bOrderKey);
              });
            }
          }
          return children;
        }
      }
    }
  }

  override getDescendants(options: { node: Entity; nodeType?: NodeType }): Entity[] {
    if (this.nodesByParent.size === 0) {
      return [];
    }

    // collect
    const nodeTypes = expandNodeTypes(options.nodeType, { expandInheritance: true });
    const queue: Entity[] = [options.node];
    const descendants: Entity[] = [];
    while (queue.length > 0) {
      const current = queue.shift()!;
      const children = this.getChildren({ node: current, nodeType: options.nodeType });
      queue.push(...children);
      // collect level
      if (nodeTypes === null) {
        descendants.push(...children);
      } else {
        descendants.push(...children.filter((child) => nodeTypes.includes(child.metatype)));
      }
    }

    return descendants;
  }
}

/** A Graph with a flat set of Events. */
export class EventGraph extends Graph<Event> {
  readonly nodesById: Map<string, Event>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this.nodesById = new Map();
  }

  override get size(): number {
    return this.nodesById.size;
  }

  override get nodes(): Event[] {
    return Array.from(this.nodesById.values());
  }

  override get(id: string): Event | null {
    return this.nodesById.get(id) || null;
  }

  override has(id: string): boolean {
    return this.nodesById.has(id);
  }

  override clear(): void {
    this.nodesById.clear();
  }

  override add(node: Event): void {
    const existing = this.nodesById.get(node.id);
    if (existing !== undefined) {
      throw new Error(`node ${node} already in ${this}: ${existing}`);
    }
    // node
    this.nodesById.set(node.id, node);
  }

  override remove(node: Event): void {
    this.nodesById.delete(node.id);
  }

  override getRoots(options?: { nodeType?: NodeType }): Event[] {
    return [];
  }

  override getLeaves(options?: { nodeType?: NodeType; node?: Event }): Event[] {
    return [];
  }

  override getChildren(options: { node: Event; nodeType?: NodeType }): Event[] {
    return [];
  }

  override getDescendants(options: { node: Event; nodeType?: NodeType }): Event[] {
    return [];
  }
}

/** A Graph with a flat set of generic Nodes. */
export class GenericGraph extends Graph<Node> {
  readonly nodesById: Map<string, Node>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this.nodesById = new Map();
  }

  override get size(): number {
    return this.nodesById.size;
  }

  override get nodes(): Node[] {
    return Array.from(this.nodesById.values());
  }

  override get(id: string): Node | null {
    return this.nodesById.get(id) || null;
  }

  override has(id: string): boolean {
    return this.nodesById.has(id);
  }

  override clear(): void {
    this.nodesById.clear();
  }

  override add(node: Node): void {
    const existing = this.nodesById.get(node.id);
    if (existing !== undefined) {
      throw new Error(`node ${node} already in ${this}: ${existing}`);
    }
    // node
    this.nodesById.set(node.id, node);
  }

  override remove(node: Node): void {
    this.nodesById.delete(node.id);
  }

  override getRoots(options?: { nodeType?: NodeType }): Node[] {
    return [];
  }

  override getLeaves(options?: { nodeType?: NodeType; node?: Node }): Node[] {
    return [];
  }

  override getChildren(options: { node: Node; nodeType?: NodeType }): Node[] {
    return [];
  }

  override getDescendants(options: { node: Node; nodeType?: NodeType }): Node[] {
    return [];
  }
}

/** An always empty Graph. */
export class NullGraph extends Graph<Node> {
  override get size(): number {
    return 0;
  }

  override get nodes(): Node[] {
    return [];
  }

  override get(id: string): Node | null {
    return null;
  }

  override has(id: string): boolean {
    return false;
  }

  override clear(): void {
    // do nothing
  }

  override add(node: Node): void {
    throw new Error("cannot add a Node to a NullGraph");
  }

  override remove(node: Node): void {
    throw new Error("cannot remove a Node from a NullGraph");
  }

  override getRoots(): Node[] {
    return [];
  }

  override getLeaves(): Node[] {
    return [];
  }

  override getChildren(options: { node: Node; nodeType?: NodeType }): Node[] {
    return [];
  }

  override getDescendants(options: { node: Node; nodeType?: NodeType }): Node[] {
    return [];
  }
}

const _MISSING = Symbol("missing");

/** A Supergraph is a collection of Graphs. */
export class Supergraph {
  readonly session: Session;
  readonly graphs: Graph[];
  readonly _cachedNodesById: Map<string, Node | typeof _MISSING>;

  constructor(session: Session) {
    this.session = session;
    this.graphs = [];
    this._cachedNodesById = new Map();
  }

  repr(): string {
    return `<Supergraph ${this.graphs.length} graphs>`;
  }

  /** Create a new EntitySingletonGraph and add it to this Supergraph. */
  createEntitySingletonGraph(node: Entity): EntitySingletonGraph {
    const newGraph = new EntitySingletonGraph(this, node);
    this.addGraph(newGraph);
    return newGraph;
  }

  /** Create a new EntityGraph and add it to this Supergraph. */
  createEntityGraph(): EntityGraph {
    const newGraph = new EntityGraph(this);
    this.addGraph(newGraph as Graph<Entity>);
    return newGraph;
  }

  /** Create a new EventGraph and add it to this Supergraph. */
  createEventGraph(): EventGraph {
    const newGraph = new EventGraph(this);
    this.addGraph(newGraph);
    return newGraph;
  }

  /** Promote an EntitySingletonGraph to an EntityGraph in one operation. */
  promoteToPolygraph(graph: EntitySingletonGraph): EntityGraph {
    const newGraph = this.createEntityGraph();
    newGraph.add(graph.node);
    this.graphs.splice(this.graphs.indexOf(graph), 1);
    this.graphs.push(newGraph as Graph<Entity>);
    return newGraph;
  }

  /** Add a Graph to this Supergraph. */
  addGraph(graph: Graph): void {
    this.graphs.push(graph);
    for (const node of graph.nodes) {
      const cached = this._cachedNodesById.get(node.id);
      if (cached === undefined || cached === _MISSING) {
        this._cachedNodesById.set(node.id, node);
      }
    }
  }

  /** Remove a Graph from this Supergraph. */
  removeGraph(graph: Graph): void {
    this.graphs.splice(this.graphs.indexOf(graph), 1);
    for (const node of graph.nodes) {
      if (this._cachedNodesById.get(node.id) === node) {
        this._cachedNodesById.delete(node.id);
      }
    }
  }

  /** Get a node by ID. */
  get(nodeId: string): Node | null {
    const cached = this._cachedNodesById.get(nodeId);
    if (cached === undefined) {
      // look in all graphs
      for (const graph of this.graphs) {
        const node = graph.get(nodeId);
        if (node !== null) {
          this._cachedNodesById.set(nodeId, node);
          return node;
        }
      }
      this._cachedNodesById.set(nodeId, _MISSING);
      return null;
    } else if (cached === _MISSING) {
      return null;
    } else {
      return cached;
    }
  }

  /** Get a node by ID (error if not found). */
  getOrError(nodeId: string): Node {
    const node = this.get(nodeId);
    if (node === null) {
      throw new Error(`node ${nodeId} not found in ${this.repr()}`);
    }
    return node;
  }
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
