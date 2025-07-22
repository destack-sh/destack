import { signal, Signal } from "@preact/signals-react";
import {
  Entity,
  EntityGraph,
  EntitySingletonGraph,
  Graph,
  Node,
  NodeType,
  Supergraph,
} from "destack";

/** A reactive Graph. */
export interface ReactiveGraph<T extends Node> extends Graph<T> {
  /** Touch a Node reactively. */
  touch(id: string): void;

  /** 'Subscribe' to a Node reactively. */
  subscribe(id: string): void;

  /** Touch the children of a Node reactively. */
  touchChildren(id: string): void;

  /** 'Subscribe' to the children of a Node reactively. */
  subscribeChildren(id: string): void;
}

/** A reactive variant of an EntitySingletonGraph. */
export class ReactiveEntitySingletonGraph
  extends EntitySingletonGraph
  implements ReactiveGraph<Entity>
{
  readonly _signal: Signal<number>;

  constructor(supergraph: Supergraph, node: Entity) {
    super(supergraph, node);
    this._signal = signal(0);
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

  override get(id: string): Entity | null {
    this.subscribe(id);
    return super.get(id);
  }
}

/** A reactive variant of an EntityGraph. */
export class ReactiveEntityGraph extends EntityGraph implements ReactiveGraph<Entity> {
  readonly _signalById: Map<string, Signal<number>>;
  readonly _signalByParent: Map<string, Signal<number>>;

  constructor(supergraph: Supergraph) {
    super(supergraph);
    this._signalById = new Map();
    this._signalByParent = new Map();
  }

  touch(id: string): void {
    if (this._signalById.has(id)) {
      this._signalById.get(id)!.value += 1;
    }
    const node = this.nodesById.get(id);
    if (node?.parentPtr != null) {
      this.touchChildren(node.parentPtr.id);
    }
  }

  subscribe(id: string): void {
    if (this.nodesById.has(id)) {
      if (!this._signalById.has(id)) {
        this._signalById.set(id, signal(0));
      }
      this._signalById.get(id)!.value;
    }
  }

  touchChildren(id: string): void {
    if (this._signalByParent.has(id)) {
      this._signalByParent.get(id)!.value += 1;
    }
  }

  subscribeChildren(id: string): void {
    if (this._signalByParent.has(id)) {
      this._signalByParent.get(id)!.value;
    }
  }

  override get(id: string): Entity | null {
    this.subscribe(id);
    return super.get(id);
  }

  override has(id: string): boolean {
    this.subscribe(id);
    return super.has(id);
  }

  override clear(): void {
    super.clear();
    this._signalById.clear();
    this._signalByParent.clear();
  }

  override add(node: Entity): void {
    this.touch(node.id);
    super.add(node);
  }

  override remove(node: Entity): void {
    this.touch(node.id);
    super.remove(node);
    this._signalById.delete(node.id);
    this._signalByParent.delete(node.id);
  }

  override getRoots(options?: { nodeType?: NodeType }): Entity[] {
    return super.getRoots(options);
  }

  override getLeaves(options?: { nodeType?: NodeType }): Entity[] {
    return super.getLeaves(options);
  }

  override getChildren(options: { node: Entity; nodeType?: NodeType }): Entity[] {
    this.subscribeChildren(options.node.id);
    return super.getChildren(options);
  }
}

/** A reactive variant of Supergraph. */
export class ReactiveSupergraph extends Supergraph {
  override createEntitySingletonGraph(node: Entity): ReactiveEntitySingletonGraph {
    const newGraph = new ReactiveEntitySingletonGraph(this, node);
    this.addGraph(newGraph);
    return newGraph;
  }

  override createEntityGraph(): ReactiveEntityGraph {
    const newGraph = new ReactiveEntityGraph(this);
    this.addGraph(newGraph);
    return newGraph;
  }
}
