import {
  Change,
  ChangeResult,
  Edit,
  IsSubject,
  Node,
  Oracle,
  Origin,
  QueryConnection,
  Space,
  Store,
  Supergraph,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/**
 * A managed Session for interacting with Destack.
 */
export class Session {
  changes: Change[];
  closedAt: Temporal.ZonedDateTime | null;
  connections: QueryConnection[];
  dirty: Record<string, Node>;
  edits: Edit[];
  oracle: Oracle;
  origin: Origin | null;
  space: Space | null;
  store: Store | null;
  subject: (Node & IsSubject) | null;
  supergraph: Supergraph;

  constructor(
    oracle: Oracle,
    space: Space | null,
    origin: Origin | null,
    subject: (Node & IsSubject) | null,
    store: Store | null,
  ) {
    this.oracle = oracle;
    this.space = space;
    this.origin = origin;
    this.subject = subject;
    this.store = store;
    this.supergraph = new Supergraph(this);

    // transaction (pending)
    this.dirty = {};
    this.edits = [];
    this.changes = [];

    // runtime
    this.connections = [];
    this.closedAt = null;
  }

  repr(): string {
    const contentParts: string[] = [];
    if (this.space) {
      contentParts.push(`space=${this.space.slug}`);
    }
    if (this.subject) {
      contentParts.push(`subject=${this.subject.repr()}`);
    }
    if (this.store) {
      contentParts.push(`store=${this.store.repr()}`);
    }
    if (this.closedAt) {
      contentParts.push(`closed_at=${this.closedAt.toString()}`);
    }
    return `Session(${contentParts.join(", ")})`;
  }

  /**
   * Open the Session.
   */
  async open(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is already closed`);
    }
  }

  /**
   * Close the Session.
   */
  async close(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is already closed`);
    }
  }

  /** Create a new Node. */
  create(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Create or update a Node. */
  upsert(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Update a Node. */
  update(node: Node, edit: Edit): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Move a Node to a new parent. */
  move(node: Node, parent: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Archive a Node. */
  archive(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Unarchive a Node. */
  unarchive(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Delete a Node. */
  delete(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Restore a deleted Node. */
  restore(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Erase a Node. */
  erase(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Turn pending updates into Edits, and Edits into Changes. */
  flush(): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Stage pending Edits. Also stages pending Changes in the Store if possible. */
  async stage(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }

  /** Commit all Changes/Edits. Returns applied Changes. */
  async commit(): Promise<ChangeResult[]> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    throw new Error("not implemented");
  }
}
