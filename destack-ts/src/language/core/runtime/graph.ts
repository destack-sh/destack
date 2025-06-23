import { NodeTypeMapping, TraitTypeMapping } from "@/language/registry";
import { Node, NodeType, TraitType } from "../builtin";
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
  abstract getRoots<N extends Node>(options: { nodeClass: new (...args: any[]) => N }): N[];
  abstract getRoots<N extends Node = Node>(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: new (...args: any[]) => N;
  }): N[];

  /** Find leaf Nodes in the graph. */
  abstract getLeaves(options?: { of?: Node }): Node[];
  abstract getLeaves<T extends NodeType>(options: { nodeType: T; of?: Node }): NodeTypeMapping[T][];
  abstract getLeaves<T extends TraitType>(options: { traitType: T; of?: Node }): (Node & TraitTypeMapping[T])[];
  abstract getLeaves<N extends Node>(options: { nodeClass: new (...args: any[]) => N; of?: Node }): N[];
  abstract getLeaves<N extends Node = Node>(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: new (...args: any[]) => N;
    of?: Node;
  }): N[];

  /**
   * Collect child Nodes (one level down).
   * If the Nodes are IsOrdered, their order is preserved.
   */
  abstract getChildren(node: Node): Node[];
  abstract getChildren<T extends NodeType>(node: Node, options: { nodeType: T }): NodeTypeMapping[T][];
  abstract getChildren<T extends TraitType>(node: Node, options: { traitType: T }): (Node & TraitTypeMapping[T])[];
  abstract getChildren<N extends Node>(node: Node, options: { nodeClass: new (...args: any[]) => N }): N[];
  abstract getChildren<N extends Node = Node>(
    node: Node,
    options?: {
      nodeType?: NodeType;
      traitType?: TraitType;
      nodeClass?: new (...args: any[]) => N;
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
  abstract getDescendants<N extends Node>(node: Node, options: { nodeClass: new (...args: any[]) => N }): N[];
  abstract getDescendants<N extends Node = Node>(
    node: Node,
    options?: {
      nodeType?: NodeType;
      traitType?: TraitType;
      nodeClass?: new (...args: any[]) => N;
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

  override getRoots(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: new (...args: any[]) => Node;
  }): Node[] {
    throw new Error("not implemented");
  }

  override getLeaves(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: new (...args: any[]) => Node;
    of?: Node;
  }): Node[] {
    throw new Error("not implemented");
  }

  override getChildren(node: Node, options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: new (...args: any[]) => Node;
  }): Node[] {
    throw new Error("not implemented");
  }

  override getDescendants(node: Node, options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: new (...args: any[]) => Node;
  }): Node[] {
    if (this.nodesByParent.size === 0) {
      return [];
    }

    throw new Error("not implemented");
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