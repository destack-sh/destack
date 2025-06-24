import { NodeTypeMapping, TraitTypeMapping } from "@destack/language/mapping";
import { NODE_TYPES_BY_TRAIT_TYPE } from "@destack/language/registry";
import { INTEGER_ZERO } from "@destack/utils/fractional";
import { IsOrdered, Node, NodeClass, NodeType, TraitType } from "../builtin";
import { Session } from "./session";

/** A Graph is a collection of Nodes. */
export abstract class Graph {
  readonly supergraph: Supergraph;

  constructor(supergraph: Supergraph) {
    this.supergraph = supergraph;
  }

  repr(): string {
    return `<${this.constructor.name} ${this.nodes.length} nodes>`;
  }

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
  abstract getRoots<T extends NodeType>(options: { nodeType: T }): NodeTypeMapping[T][];
  abstract getRoots<T extends TraitType>(options: { traitType: T }): (Node & TraitTypeMapping[T])[];
  abstract getRoots<N extends Node>(options: { nodeClass: NodeClass }): N[];
  abstract getRoots<N extends Node = Node>(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
  }): N[];

  /** Find leaf Nodes in the graph. */
  abstract getLeaves(options?: { node?: Node }): Node[];
  abstract getLeaves<T extends NodeType>(options: { nodeType: T; node?: Node }): NodeTypeMapping[T][];
  abstract getLeaves<T extends TraitType>(options: { traitType: T; node?: Node }): (Node & TraitTypeMapping[T])[];
  abstract getLeaves<N extends Node>(options: { nodeClass: NodeClass; node?: Node }): N[];
  abstract getLeaves<N extends Node = Node>(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
    node?: Node;
  }): N[];

  /**
   * Collect child Nodes (one level down).
   * If the Nodes are IsOrdered, their order is preserved.
   */
  abstract getChildren(node: Node): Node[];
  abstract getChildren<T extends NodeType>(node: Node, options: { nodeType: T }): NodeTypeMapping[T][];
  abstract getChildren<T extends TraitType>(node: Node, options: { traitType: T }): (Node & TraitTypeMapping[T])[];
  abstract getChildren<N extends Node>(node: Node, options: { nodeClass: NodeClass }): N[];
  abstract getChildren<N extends Node = Node>(
    node: Node,
    options?: {
      nodeType?: NodeType;
      traitType?: TraitType;
      nodeClass?: NodeClass;
    },
  ): N[];

  /**
   * Collect descendant Nodes (recursively down).
   * If a type is specified, only Nodes of that type are collected.
   * (Descendants are not collected unless all their ancestors are included).
   * Nodes are BFS but IsOrdered is ignored.
   */
  abstract getDescendants(node: Node): Node[];
  abstract getDescendants<T extends NodeType>(node: Node, options: { nodeType: T }): NodeTypeMapping[T][];
  abstract getDescendants<T extends TraitType>(node: Node, options: { traitType: T }): (Node & TraitTypeMapping[T])[];
  abstract getDescendants<N extends Node>(node: Node, options: { nodeClass: NodeClass }): N[];
  abstract getDescendants<N extends Node = Node>(
    node: Node,
    options?: {
      nodeType?: NodeType;
      traitType?: TraitType;
      nodeClass?: NodeClass;
    },
  ): N[];
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

  override getRoots(options?: { nodeType?: NodeType; traitType?: TraitType; nodeClass?: NodeClass }): Node[] {
    const nodeTypes = getNodeTypes(options);
    if (nodeTypes === null) {
      return this.nodes.filter((node) => node.parentPtr === null);
    }
    return this.nodes.filter((node) => node.parentPtr === null && nodeTypes.includes(node.metatype));
  }

  override getLeaves(options?: {
    node?: Node;
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
  }): Node[] {
    if (options?.node === undefined) {
      const nodeTypes = getNodeTypes(options);
      if (nodeTypes === null) {
        return this.nodes.filter((node) => !this.nodesByParent.has(node.id));
      }
      return this.nodes.filter((node) => !this.nodesByParent.has(node.id) && nodeTypes.includes(node.metatype));
    } else {
      const descendants = this.getDescendants(options.node, options);
      return descendants.filter((node) => !this.nodesByParent.has(node.id));
    }
  }

  override getChildren(
    node: Node,
    options?: {
      nodeType?: NodeType;
      traitType?: TraitType;
      nodeClass?: NodeClass;
    },
  ): Node[] {
    // bail if no children
    if (this.nodesByParent.size === 0) {
      return [];
    }
    const childrenByType = this.nodesByParent.get(node.id);
    if (!childrenByType) {
      return [];
    }

    if (options === undefined) {
      // collect children across all types
      const children: Node[] = [];
      let isOrdered = false;
      for (const childrenOfType of childrenByType.values()) {
        const nodeClass = childrenOfType[0].constructor as NodeClass;
        if (nodeClass.__traits__.includes(TraitType.ORDERED)) {
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
      const nodeTypes = getNodeTypes(options);
      if (nodeTypes === null) {
        // collect children across all types
        const children: Node[] = [];
        let isOrdered = false;
        for (const childrenOfType of childrenByType.values()) {
          const nodeClass = childrenOfType[0].constructor as NodeClass;
          if (nodeClass.__traits__.includes(TraitType.ORDERED)) {
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
            const nodeClass = children[0].constructor as NodeClass;
            if (nodeClass.__traits__.includes(TraitType.ORDERED)) {
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
            const nodeClass = children[0].constructor as NodeClass;
            if (nodeClass.__traits__.includes(TraitType.ORDERED)) {
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

  override getDescendants(
    node: Node,
    options?: {
      nodeType?: NodeType;
      traitType?: TraitType;
      nodeClass?: NodeClass;
    },
  ): Node[] {
    if (this.nodesByParent.size === 0) {
      return [];
    }

    const queue: Node[] = [node];
    const descendants: Node[] = [];

    // collect
    const nodeTypes = getNodeTypes(options);
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

/** Resolve the NodeTypes for a NodeType, TraitType, or Node class. */
export function getNodeTypes(options?: {
  nodeType?: NodeType;
  traitType?: TraitType;
  nodeClass?: NodeClass;
}): NodeType[] | null {
  if (options == null) {
    return null;
  }
  const nodeTypes: NodeType[] = [];
  if (options?.nodeType) {
    nodeTypes.push(options.nodeType);
  }
  if (options?.traitType) {
    nodeTypes.push(...NODE_TYPES_BY_TRAIT_TYPE[options.traitType]);
  }
  if (options?.nodeClass) {
    nodeTypes.push(options.nodeClass.metatype);
  }
  return nodeTypes;
}
