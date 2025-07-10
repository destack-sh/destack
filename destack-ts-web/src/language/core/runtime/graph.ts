import { signal, Signal } from "@preact/signals-react";
import {
  expandNodeTypes,
  Graph,
  Node,
  NodeType,
  PolyGraph,
  SingletonGraph,
  Supergraph,
} from "destack";

/** A reactive Graph. */
export interface ReactiveGraph extends Graph {
  /** Touch all Nodes reactively. */
  touchAll(): void;

  /** Subscribe to all Nodes reactively. */
  subscribeAll(): void;

  /** Touch a Node reactively. */
  touch(id: string): void;

  /** 'Subscribe' to a Node reactively. */
  subscribe(id: string): void;

  /** Touch the children of a Node reactively. */
  touchChildren(id: string): void;

  /** 'Subscribe' to the children of a Node reactively. */
  subscribeChildren(id: string): void;
}

/** A reactive variant of SingletonGraph. */
export class ReactiveSingletonGraph extends SingletonGraph implements ReactiveGraph {
  readonly _signal: Signal<number>;

  constructor(supergraph: Supergraph, node: Node) {
    super(supergraph, node);
    this._signal = signal(0);
  }

  touchAll(): void {
    this._signal.value++;
  }

  subscribeAll(): void {
    this._signal.value;
  }

  touch(id: string): void {
    if (this.node.id == id) {
      this._signal.value += 1;
    }
  }

  subscribe(id: string): void {
    if (this.node.id == id) {
      this._signal.value;
    }
  }

  touchChildren(id: string): void {
    // nothing to do
  }

  subscribeChildren(id: string): void {
    // nothing to do
  }

  override get(id: string): Node | null {
    this.subscribe(id);
    return super.get(id);
  }

  override getRoots(): Node[] {
    this.subscribeAll();
    return super.getRoots();
  }

  override getLeaves(): Node[] {
    this.subscribeAll();
    return super.getLeaves();
  }
}

/** A reactive variant of PolyGraph. */
export class ReactivePolyGraph extends PolyGraph implements ReactiveGraph {
  readonly _signalAll: Signal<number>;
  readonly _signalById: Map<string, Signal<number>>;
  readonly _signalByParent: Map<string, Signal<number>>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this._signalAll = signal(0);
    this._signalById = new Map();
    this._signalByParent = new Map();
  }

  touchAll(): void {
    console.log("ReactiveGraph.touchAll");
    this._signalAll.value++;
  }

  subscribeAll(): void {
    console.log("ReactiveGraph.subscribeAll");
    this._signalAll.value;
  }

  touch(id: string): void {
    console.log("ReactiveGraph.touch", id);
    this._signalAll.value++;
    if (this._signalById.has(id)) {
      this._signalById.get(id)!.value += 1;
    }
    const node = this.nodesById.get(id);
    if (node?.parentPtr != null) {
      this.touchChildren(node.parentPtr.id);
    }
  }

  subscribe(id: string): void {
    console.log("ReactiveGraph.subscribe", id);
    this._signalAll.value;
    if (this.nodesById.has(id)) {
      if (!this._signalById.has(id)) {
        this._signalById.set(id, signal(0));
      }
      this._signalById.get(id)!.value;
    }
  }

  touchChildren(id: string): void {
    console.log("ReactiveGraph.touchChildren", id);
    this._signalAll.value++;
    if (this._signalByParent.has(id)) {
      this._signalByParent.get(id)!.value += 1;
    }
  }

  subscribeChildren(id: string): void {
    console.log("ReactiveGraph.subscribeChildren", id);
    this._signalAll.value;
    if (this._signalByParent.has(id)) {
      this._signalByParent.get(id)!.value;
    }
  }

  override get size(): number {
    this.subscribeAll();
    return this.nodes.length;
  }

  override get nodes(): Node[] {
    this.subscribeAll();
    return super.nodes;
  }

  override get(id: string): Node | null {
    this.subscribe(id);
    return super.get(id);
  }

  override has(id: string): boolean {
    this.subscribe(id);
    return super.has(id);
  }

  override clear(): void {
    this.touchAll();
    super.clear();
    this._signalById.clear();
    this._signalByParent.clear();
  }

  override add(node: Node): void {
    this.touch(node.id);
    super.add(node);
  }

  override remove(node: Node): void {
    this.touch(node.id);
    super.remove(node);
    this._signalById.delete(node.id);
    this._signalByParent.delete(node.id);
  }

  override getRoots(options?: { nodeType?: NodeType }): Node[] {
    this.subscribeAll();
    return super.getRoots(options);
  }

  override getLeaves(options?: { nodeType?: NodeType }): Node[] {
    this.subscribeAll();
    return super.getLeaves(options);
  }

  override getChildren(options: { node: Node; nodeType?: NodeType }): Node[] {
    this.subscribeChildren(options.node.id);
    return super.getChildren(options);
  }

  override getDescendants(options: { node: Node; nodeType?: NodeType }): Node[] {
    if (this.nodesByParent.size === 0) {
      return [];
    }

    // collect
    const nodeTypes = expandNodeTypes(options.nodeType, { expandInheritance: true });
    const queue: Node[] = [options.node];
    const descendants: Node[] = [];
    while (queue.length > 0) {
      const current = queue.shift()!;
      const childrenByType = this.nodesByParent.get(current.id);
      this.subscribeChildren(current.id);
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

/** A reactive variant of Supergraph. */
export class ReactiveSupergraph extends Supergraph {
  override createSingletonGraph(node: Node): ReactiveSingletonGraph {
    const newGraph = new ReactiveSingletonGraph(this, node);
    this.addGraph(newGraph);
    return newGraph;
  }

  override createPolyGraph(): ReactivePolyGraph {
    const newGraph = new ReactivePolyGraph(this);
    this.addGraph(newGraph);
    return newGraph;
  }
}
