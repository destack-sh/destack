import { signal, Signal } from "@preact/signals-react";
import { Node, NodeType, PolyGraph, SingletonGraph, Supergraph } from "destack";

interface ReactiveGraph {
  /** Touch a Node reactively. */
  touch(id: string): void;

  /** 'Subscribe' to a Node reactively. */
  subscribe(id: string): void;

  /** Touch all Nodes reactively. */
  touchAll(nodeType: NodeType | null): void;

  /** Subscribe to all Nodes reactively. */
  subscribeAll(nodeType: NodeType | null): void;

  /** Touch the children of a Node reactively. */
  touchChildren(id: string, nodeType: NodeType | null): void;

  /** 'Subscribe' to the children of a Node reactively. */
  subscribeChildren(id: string, nodeType: NodeType | null): void;
}

/** A reactive variant of SingletonGraph. */
export class ReactiveSingletonGraph extends SingletonGraph implements ReactiveGraph {
  readonly _signal: Signal<number>;

  constructor(supergraph: Supergraph, node: Node) {
    super(supergraph, node);
    this._signal = signal(0);
  }

  touch(id: string): void {
    if (this.node.id == id) {
      this._signal.value++;
    }
  }

  subscribe(id: string): void {
    if (this.node.id == id) {
      this._signal.value;
    }
  }

  touchAll(nodeType: NodeType | null): void {
    if (nodeType === null || this.node.__inherits__.includes(nodeType)) {
      this._signal.value++;
    }
  }

  subscribeAll(nodeType: NodeType | null): void {
    if (nodeType === null || this.node.__inherits__.includes(nodeType)) {
      this._signal.value;
    }
  }

  touchChildren(id: string, nodeType: NodeType): void {
    // nothing to do
  }

  subscribeChildren(id: string, nodeType: NodeType): void {
    // nothing to do
  }

  override get(id: string): Node | null {
    this.subscribe(id);
    return super.get(id);
  }

  override getRoots(): Node[] {
    this.subscribeAll(null);
    return super.getRoots();
  }

  override getLeaves(): Node[] {
    this.subscribeAll(null);
    return super.getLeaves();
  }
}

/** A reactive variant of PolyGraph. */
export class ReactivePolyGraph extends PolyGraph implements ReactiveGraph {
  readonly _signalById: Map<string, Signal<number>>;
  readonly _signalByParent: Map<string, Map<NodeType, Signal<number>>>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this._signalById = new Map();
    this._signalByParent = new Map();
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
