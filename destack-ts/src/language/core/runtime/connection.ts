import type { Event, Graph, NodeReference, Session } from "@destack/language";

/**
 * A connection between a local and a remote Graph.
 */
export class Connection {
  /** The Session this Connection is in. */
  readonly session: Session;

  /** The local Graph. */
  readonly graph: Graph;

  /** The remote Space. */
  readonly remoteSpaceRef: NodeReference;

  constructor(options: { remoteSpaceRef: NodeReference; graph: Graph; session: Session }) {
    this.remoteSpaceRef = options.remoteSpaceRef;
    this.session = options.session;
    this.graph = options.graph;
  }

  repr(): string {
    return `<Connection remote=${this.remoteSpaceRef.id}>`;
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
