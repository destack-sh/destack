import { TypeCardinality } from "@destack/language/core/builtin/common";
import { ACTIVE_SESSION } from "@destack/language/core/builtin/const";
import type { Node, NodeClass } from "@destack/language/core/builtin/node";
import type { IsSubject } from "@destack/language/core/builtin/trait";
import {
  Change,
  ChangeResult,
  ChangeStatus,
  Edit,
  EditOperation,
  EditType,
  Origin,
} from "@destack/language/core/common/edit";
import { toValue, Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import { Supergraph } from "@destack/language/core/runtime/graph";
import { WORLD_ORACLE, type Oracle } from "@destack/language/core/runtime/oracle";
import type { Store } from "@destack/language/core/runtime/store";
import type { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/**
 * A managed Session for interacting with Destack.
 */
export class Session {
  oracle: Oracle;
  space: Space | null;
  origin: Origin | null;
  subject: (Node & IsSubject) | null;
  store: Store | null;
  supergraph: Supergraph;

  dirty: Record<string, Node>;
  edits: Edit[];
  changes: Change[];

  connections: QueryConnection[];
  closedAt: Temporal.ZonedDateTime | null;
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
    const edit = new Edit({ type: EditType.CREATE, node, value: toValue(node, null, true) });
    this.edits.push(edit);
    this.dirty[node.id] = node;
    node._isNew = false;
    node._isAttached = true;
  }

  /** Create or update a Node. */
  upsert(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new Edit({ type: EditType.UPSERT, node, value: toValue(node, null, true) });
    this.edits.push(edit);
    this.dirty[node.id] = node;
    node._isNew = false;
    node._isAttached = true;
  }

  /** Update a Node. */
  update(node: Node, edit: Edit): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Move a Node to a new parent. */
  move(node: Node, parent: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    const edit = new Edit({ type: EditType.MOVE, node, value: toValue(parent) });
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Archive a Node. */
  archive(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    const undoEdit = new Edit({ type: EditType.RESTORE, node, value: toValue(node, null, true) });
    const edit = new Edit({ type: EditType.ARCHIVE, node, undo: undoEdit });
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Unarchive a Node. */
  unarchive(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    const edit = new Edit({ type: EditType.UNARCHIVE, node });
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Delete a Node. */
  delete(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    const undoEdit = new Edit({ type: EditType.RESTORE, node, value: toValue(node, null, true) });
    const edit = new Edit({ type: EditType.DELETE, node, undo: undoEdit });
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Restore a deleted Node. */
  restore(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    const edit = new Edit({ type: EditType.RESTORE, node });
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Erase a Node. */
  erase(node: Node): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._flushNode(node);
    const undoEdit = new Edit({ type: EditType.CREATE, node, value: toValue(node, null, true) });
    const edit = new Edit({ type: EditType.ERASE, node, undo: undoEdit });
    this.edits.push(edit);
    this.dirty[node.id] = node;
  }

  /** Turn a dirty Node into Edits. */
  _flushNode(node: Node): void {
    if (node._isNew) {
      node._isNew = false;
    } else if (node._dirty != null) {
      // turn dirty properties into Edits (basic SET/CLEAR operations)
      const nodePtr = node.toRef();
      for (const [propName, propOldValue] of Object.entries(node._dirty)) {
        const prop = (node.constructor as NodeClass).__properties__[propName];
        const propPtr = prop.toRef();
        const propType = prop.toType();

        // undo
        let undoOperation: EditOperation;
        let oldValue: Value | null;
        if (propOldValue == null || (prop.cardinality != TypeCardinality.SCALAR && !propOldValue)) {
          undoOperation = EditOperation.CLEAR;
          oldValue = null;
        } else {
          undoOperation = EditOperation.SET;
          oldValue = toValue(propOldValue, propType);
        }

        // do
        const propNewValue = (node as any)[propName];
        let operation: EditOperation;
        let newValue: Value | null;
        if (propNewValue == null || (prop.cardinality != TypeCardinality.SCALAR && !propNewValue)) {
          operation = EditOperation.CLEAR;
          newValue = null;
        } else {
          operation = EditOperation.SET;
          newValue = toValue(propNewValue, propType);
        }

        // create Edits
        const undoEdit = new Edit({
          type: EditType.UPDATE,
          node: nodePtr,
          propPtr: propPtr,
          operation: undoOperation,
          value: oldValue,
        });
        const edit = new Edit({
          type: EditType.UPDATE,
          node: nodePtr,
          propPtr: propPtr,
          operation: operation,
          value: newValue,
          undo: undoEdit,
        });
        this.edits.push(edit);
      }
    }
  }

  /** Turn pending updates into Edits, and Edits into Changes. */
  flush(): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    // flush dirty Nodes
    const dirtyNodes = Object.values(this.dirty);
    if (dirtyNodes.length > 0) {
      for (const node of dirtyNodes) {
        this._flushNode(node);
      }
      this.dirty = {};
    }
    // flush Edits
    if (this.edits.length > 0) {
      const change = new Change({
        edits: this.edits,
        createdBy: this.subject,
        origin: this.origin,
      });
      this.edits = [];
      this.changes.push(change);
    }
  }

  /** Stage pending Edits. Also stages pending Changes in the Store if possible. */
  async stage(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this.flush();
  }

  /** Commit all Changes/Edits. Returns applied Changes. */
  async commit(): Promise<ChangeResult[]> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    } else if (this.store == null) {
      throw new Error(`${this.repr()} has no Store`);
    }
    this.flush();
    const changes = this.changes;
    this.changes = [];
    const results = await this.store.commit(changes);
    if (results.some((result) => result.status != ChangeStatus.COMPLETED)) {
      const badResults = results.filter((result) => result.status != ChangeStatus.COMPLETED);
      throw new Error(
        `failed to commit ${changes.length} Changes: ${badResults
          .map((result) => result.id)
          .join(", ")}`,
      );
    }
    return results;
  }
}
