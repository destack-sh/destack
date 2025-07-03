import type { IsOrdered, Node, NodeClass } from "@destack/language/core/builtin";
import { NodeType, TraitType } from "@destack/language/core/builtin/common";
import { hasTrait } from "@destack/language/core/builtin/node";
import { TraitClass } from "@destack/language/core/builtin/trait";
import type { Session } from "@destack/language/core/runtime/session";
import type { TraitTypeMapping } from "@destack/language/mapping";
import { NODE_CLASS_BY_TYPE, NODE_TYPES_BY_TRAIT_TYPE } from "@destack/language/registry";
import { INTEGER_ZERO } from "@destack/utils/fractional";

/** A Graph is a collection of Nodes. */
export abstract class Graph {
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
  abstract get nodes(): Node[];

  /** Get a Node by id. */
  abstract get(id: string): Node | null;

  /** Get a Node by id, or throw an error if not found. */
  getOrError(id: string): Node {
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
  abstract add(node: Node): void;

  /** Remove a Node from the Graph (must exist, excluding descendants). */
  abstract remove(node: Node): void;

  /** Find root Nodes in the graph. */
  abstract getRoots(): Node[];
  abstract getRoots<N extends Node>(classOrTrait: NodeClass<N>): N[];
  abstract getRoots<T extends TraitType>(
    clasOrTrait: TraitClass<any, T>,
  ): (Node & TraitTypeMapping[T])[];
  abstract getRoots(classOrTrait?: NodeClass | TraitClass): Node[];

  /** Find leaf Nodes in the graph. */
  abstract getLeaves(): Node[];
  abstract getLeaves<N extends Node>(classOrTrait: NodeClass<N>): N[];
  abstract getLeaves<T extends TraitType>(
    classOrTrait: TraitClass<any, T>,
  ): (Node & TraitTypeMapping[T])[];
  abstract getLeaves(classOrTrait?: NodeClass | TraitClass): Node[];

  /**
   * Collect child Nodes (one level down).
   * If the Nodes are IsOrdered, their order is preserved.
   */
  abstract getChildren(node: Node): Node[];
  abstract getChildren<N extends Node>(node: Node, classOrTrait: NodeClass<N>): N[];
  abstract getChildren<T extends TraitType>(
    node: Node,
    classOrTrait: TraitClass<any, T>,
  ): (Node & TraitTypeMapping[T])[];
  abstract getChildren(node: Node, classOrTrait?: NodeClass | TraitClass): Node[];

  /**
   * Collect descendant Nodes (recursively down).
   * If a type is specified, only Nodes of that type are collected.
   * (Descendants are not collected unless all their ancestors are included).
   * Nodes are BFS but IsOrdered is ignored.
   */
  abstract getDescendants(node: Node): Node[];
  abstract getDescendants<N extends Node>(node: Node, classOrTrait: NodeClass<N>): N[];
  abstract getDescendants<T extends TraitType>(
    node: Node,
    classOrTrait: TraitClass<any, T>,
  ): (Node & TraitTypeMapping[T])[];
  abstract getDescendants(node: Node, classOrTrait?: NodeClass | TraitClass): Node[];
}

/** A Graph that contains only a single Node. */
export class SingletonGraph extends Graph {
  readonly node: Node;

  constructor(supergraph: Supergraph, node: Node) {
    super(supergraph);
    this.node = node;
  }

  toPolygraph(): PolyGraph {
    const newGraph = new PolyGraph(this.supergraph);
    newGraph.add(this.node);
    return newGraph;
  }

  override get size(): number {
    return 1;
  }

  override get nodes(): Node[] {
    return [this.node];
  }

  override get(id: string): Node | null {
    return this.node.id === id ? this.node : null;
  }

  override has(id: string): boolean {
    return this.node.id === id;
  }

  override clear(): void {
    throw new Error("cannot clear a SingletonGraph");
  }

  override add(node: Node): void {
    throw new Error("cannot add a Node to a SingletonGraph");
  }

  override remove(node: Node): void {
    throw new Error("cannot remove a Node from a SingletonGraph");
  }

  override getRoots(): Node[] {
    return [this.node];
  }

  override getLeaves(): Node[] {
    return [this.node];
  }

  override getChildren(node: Node): Node[] {
    return [];
  }

  override getDescendants(node: Node): Node[] {
    return [];
  }
}

/** A Graph with an arbitrary set of Nodes. */
export class PolyGraph extends Graph {
  readonly nodesById: Map<string, Node>;
  readonly nodesByParent: Map<string, Map<NodeType, Node[]>>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this.nodesById = new Map();
    this.nodesByParent = new Map();
  }

  override get size(): number {
    return this.nodes.length;
  }

  override get nodes(): Node[] {
    return Array.from(this.nodesById.values());
  }

  override get(id: string): Node | null {
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

  override add(node: Node): void {
    const existing = this.nodesById.get(node.id);
    if (existing !== undefined) {
      throw new Error(`node ${node} already in ${this}: ${existing}`);
    }
    // node
    this.nodesById.set(node.id, node);

    // parent
    if (node.parentPtr !== null) {
      if (!this.nodesByParent.has(node.parentPtr.id)) {
        this.nodesByParent.set(node.parentPtr.id, new Map());
      }
      const childNodeType = node.metatype;
      const parentMap = this.nodesByParent.get(node.parentPtr.id)!;
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

  override remove(node: Node): void {
    // supergraph
    if (this.supergraph._cachedNodesById.get(node.id) === node) {
      this.supergraph._cachedNodesById.delete(node.id);
    }

    // parent
    if (node.parentPtr !== null) {
      const parentMap = this.nodesByParent.get(node.parentPtr.id);
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
              this.nodesByParent.delete(node.parentPtr.id);
            }
          }
        }
      }
    }

    // node
    this.nodesById.delete(node.id);
  }

  override getRoots(classOrTrait?: NodeClass | TraitClass): Node[] {
    const nodeTypes = expandNodeTypes(classOrTrait);
    if (nodeTypes === null) {
      return this.nodes.filter((node) => node.parentPtr === null);
    }
    return this.nodes.filter(
      (node) => node.parentPtr === null && nodeTypes.includes(node.metatype),
    );
  }

  override getLeaves(classOrTrait?: NodeClass | TraitClass): Node[] {
    const nodeTypes = expandNodeTypes(classOrTrait);
    if (nodeTypes === null) {
      return this.nodes.filter((node) => !this.nodesByParent.has(node.id));
    }
    return this.nodes.filter(
      (node) => !this.nodesByParent.has(node.id) && nodeTypes.includes(node.metatype),
    );
  }

  override getChildren(node: Node, classOrTrait?: NodeClass | TraitClass): Node[] {
    // bail if no children
    if (this.nodesByParent.size === 0) {
      return [];
    }
    const childrenByType = this.nodesByParent.get(node.id);
    if (!childrenByType) {
      return [];
    }

    if (classOrTrait === undefined) {
      // collect children across all types
      const children: Node[] = [];
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
      const nodeTypes = expandNodeTypes(classOrTrait);
      if (nodeTypes === null) {
        // collect children across all types
        const children: Node[] = [];
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
                const aOrderKey = (a as unknown as IsOrdered).orderKey || INTEGER_ZERO;
                const bOrderKey = (b as unknown as IsOrdered).orderKey || INTEGER_ZERO;
                return aOrderKey.localeCompare(bOrderKey);
              });
            }
          }
          return children;
        } else {
          // collect for trait (multiple node types)
          const children: Node[] = [];
          for (const nodeType of nodeTypes) {
            children.push(...(childrenByType.get(nodeType) || []));
          }
          if (children.length > 0) {
            if (hasTrait(children[0], TraitType.ORDERED)) {
              children.sort((a, b) => {
                const aOrderKey = (a as unknown as IsOrdered).orderKey || INTEGER_ZERO;
                const bOrderKey = (b as unknown as IsOrdered).orderKey || INTEGER_ZERO;
                return aOrderKey.localeCompare(bOrderKey);
              });
            }
          }
          return children;
        }
      }
    }
  }

  override getDescendants(node: Node, classOrTrait?: NodeClass | TraitClass): Node[] {
    if (this.nodesByParent.size === 0) {
      return [];
    }

    const queue: Node[] = [node];
    const descendants: Node[] = [];

    // collect
    const nodeTypes = expandNodeTypes(classOrTrait);
    while (queue.length > 0) {
      const current = queue.shift()!;
      const childrenByType = this.nodesByParent.get(current.id);
      if (!childrenByType) {
        continue;
      }
      for (const childrenOfType of childrenByType.values()) {
        queue.push(...childrenOfType);
      }

      // collect level
      if (nodeTypes === null) {
        for (const childrenOfType of childrenByType.values()) {
          descendants.push(...childrenOfType);
        }
      } else {
        for (const nodeType of nodeTypes) {
          descendants.push(...(childrenByType.get(nodeType) || []));
        }
      }
    }

    return descendants;
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

  /** Promote a SingletonGraph to a PolyGraph in one operation. */
  promoteToPolygraph(graph: SingletonGraph): PolyGraph {
    const newGraph = graph.toPolygraph();
    this.graphs.splice(this.graphs.indexOf(graph), 1);
    this.graphs.push(newGraph);
    return newGraph;
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
