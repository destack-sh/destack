import { Session } from "./session";
import { Node } from "../builtin";

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
}

/** A Graph that contains only a single Node. */
export class SingletonGraph extends Graph {
  readonly node: Node;

  constructor(supergraph: Supergraph, node: Node) {
    super(supergraph);
    this.node = node;
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
}

/** A Graph with an arbitrary set of Nodes. */
export class PolyGraph extends Graph {
  readonly nodesById: Map<string, Node>;
  readonly nodesByParentId: Map<string, Map<string, Node[]>>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this.nodesById = new Map();
    this.nodesByParentId = new Map();
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
    this.nodesById.clear();
    this.nodesByParentId.clear();
  }

  override add(node: Node): void {
    throw new Error("not implemented");
  }

  override remove(node: Node): void {
    throw new Error("not implemented");
  }
}

/** A Supergraph is a collection of Graphs. */
export class Supergraph {
  readonly session: Session;
  readonly graphs: Graph[];

  constructor(session: Session) {
    this.session = session;
    this.graphs = [];
  }

  repr(): string {
    return `<Supergraph ${this.graphs.length} graphs>`;
  }

  addGraph(graph: Graph): void {
    this.graphs.push(graph);
  }

  removeGraph(graph: Graph): void {
    this.graphs.splice(this.graphs.indexOf(graph), 1);
  }

  get(node_id: string): Node | null {
    for (const graph of this.graphs) {
      const node = graph.get(node_id);
      if (node) {
        return node;
      }
    }
    return null;
  }

  getOrError(node_id: string): Node {
    const node = this.get(node_id);
    if (!node) {
      throw new Error(`Node ${node_id} not found in ${this.repr()}`);
    }
    return node;
  }
}
