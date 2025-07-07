import type {
  Entity,
  IsSubject,
  Node,
  PropertyDefinition,
  QueryConnection,
  Space,
  Store,
} from "@destack/language";
import { TypeCardinality } from "@destack/language/core/builtin/common";
import { ACTIVE_SESSION } from "@destack/language/core/builtin/const";
import {
  Change,
  ChangeResult,
  ChangeStatus,
  Edit,
  EditOperation,
  EditType,
  Origin,
} from "@destack/language/core/builtin/edit";
import { toValue, Value } from "@destack/language/core/common/value";
import { Supergraph } from "@destack/language/core/runtime/graph";
import { WORLD_ORACLE, type Oracle } from "@destack/language/core/runtime/oracle";
import { Casing, toCasing } from "@destack/utils";
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

  /** Create a new Entity. */
  create(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new Edit({ type: EditType.CREATE, node, value: toValue(node, null, true) });
    this.edits.push(edit);
    node._isNew = false;
    node._isAttached = true;
  }

  /** Create or update an Entity. */
  upsert(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new Edit({ type: EditType.UPSERT, node, value: toValue(node, null, true) });
    this.edits.push(edit);
    node._isNew = false;
    node._isAttached = true;
  }

  /** Update an Entity. */
  update(node: Entity, edit: Edit): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this.edits.push(edit);
  }

  /** Set a Property on an Entity (direct SET/CLEAR operations). */
  updateSetProperty(node: Entity, prop: PropertyDefinition, newValue: any): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const propName = toCasing(prop.name, Casing.CAMEL);
    const nodePtr = node.toRef();
    const propPtr = prop.toRef();
    const propType = prop.toType();

    // undo
    let undoOperation: EditOperation;
    let oldValue: Value | null = (node as any)[propName];
    if (oldValue == null || (prop.cardinality != TypeCardinality.SCALAR && !oldValue)) {
      undoOperation = EditOperation.CLEAR;
      oldValue = null;
    } else {
      undoOperation = EditOperation.SET;
      oldValue = toValue(oldValue, propType);
    }

    // do
    let operation: EditOperation;
    if (newValue == null || (prop.cardinality != TypeCardinality.SCALAR && !newValue)) {
      operation = EditOperation.CLEAR;
      newValue = null;
    } else {
      operation = EditOperation.SET;
      newValue = toValue(newValue, propType);
    }

    // create Edits
    const undoEdit = new Edit({
      type: EditType.UPDATE,
      node: nodePtr,
      attribute: propPtr,
      operation: undoOperation,
      value: oldValue,
    });
    const edit = new Edit({
      type: EditType.UPDATE,
      node: nodePtr,
      attribute: propPtr,
      operation: operation,
      value: newValue,
      undo: undoEdit,
    });
    this.edits.push(edit);
  }

  /** Move a Node to a new parent. */
  move(node: Entity, parent: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new Edit({ type: EditType.MOVE, node, value: toValue(parent) });
    this.edits.push(edit);
  }

  /** Archive an Entity. */
  archive(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const undoEdit = new Edit({ type: EditType.RESTORE, node, value: toValue(node, null, true) });
    const edit = new Edit({ type: EditType.ARCHIVE, node, undo: undoEdit });
    this.edits.push(edit);
  }

  /** Unarchive an Entity. */
  unarchive(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new Edit({ type: EditType.UNARCHIVE, node });
    this.edits.push(edit);
  }

  /** Delete an Entity. */
  delete(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const undoEdit = new Edit({ type: EditType.RESTORE, node, value: toValue(node, null, true) });
    const edit = new Edit({ type: EditType.DELETE, node, undo: undoEdit });
    this.edits.push(edit);
  }

  /** Restore a deleted Entity. */
  restore(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new Edit({ type: EditType.RESTORE, node });
    this.edits.push(edit);
  }

  /** Flush pending Edits and Changes. */
  flush(): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
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

  /** Stage pending Edits and Changes. */
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
