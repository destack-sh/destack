import {
  ACTIVE_SESSION,
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
  WORLD_ORACLE,
} from "@destack/language";
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
  _token: string | null;

  constructor(options?: {
    oracle?: Oracle;
    space?: Space | null;
    origin?: Origin | null;
    subject?: (Node & IsSubject) | null;
    store?: Store | null;
  }) {
    this.oracle = options?.oracle ?? WORLD_ORACLE;
    this.space = options?.space ?? null;
    this.origin = options?.origin ?? null;
    this.subject = options?.subject ?? null;
    this.store = options?.store ?? null;
    this.supergraph = new Supergraph(this);

    // transaction (pending)
    this.dirty = {};
    this.edits = [];
    this.changes = [];

    // runtime
    this.connections = [];
    this.closedAt = null;
    this._token = null;
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
    this._token = ACTIVE_SESSION.set(this);
  }

  /**
   * Close the Session.
   */
  async close(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is already closed`);
    }
    ACTIVE_SESSION.reset(this._token);
    this._token = null;
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
