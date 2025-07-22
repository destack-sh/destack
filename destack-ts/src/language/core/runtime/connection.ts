import type { Event, Graph, Node, NodeReference, Session } from "@destack/language";

/**
 * A connection between two Graphs.
 */
export class GraphConnection<NodeT extends Node = Node> {
  readonly spacePtr: NodeReference;
  readonly session: Session;
  readonly graph: Graph;

  constructor(options: { spacePtr: NodeReference; graph: Graph; session: Session }) {
    this.spacePtr = options.spacePtr;
    this.session = options.session;
    this.graph = options.graph;
  }

  repr(): string {
    return `<GraphConnection space=${this.spacePtr.repr()}>`;
  }

  async open(): Promise<void> {
    throw new Error("not implemented");
  }

  async commit(events: Event[]): Promise<Event[]> {
    throw new Error("not implemented");
  }

  async close(): Promise<void> {
    throw new Error("not implemented");
  }
}
