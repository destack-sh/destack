import {
  Client,
  IsActor,
  type Entity,
  type Event,
  type GraphConnection,
  type PropertyDefinition,
} from "@destack/language";
import { ACTIVE_SESSION } from "@destack/language/core/builtin/const";
import { EditEvent, EditOperation, EditType } from "@destack/language/core/builtin/edit";
import { toValue, Value } from "@destack/language/core/common/value";
import { Graph } from "@destack/language/core/runtime/graph";
import { WORLD_ORACLE, type Oracle } from "@destack/language/core/runtime/oracle";
import { Casing, toCasing } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/**
 * A managed Session for interacting with Destack.
 */
export class Session {
  oracle: Oracle;
  client: Client | null;
  clientNonce: string | null;
  actor: (Entity & IsActor) | null;
  graph: Graph;

  pendingEvents: Event[];
  connections: GraphConnection[];

  closedAt: Temporal.ZonedDateTime | null;
  _token: string | null;
  _epoch: number | null;

  constructor(options: {
    oracle?: Oracle;
    client?: Client | null;
    clientNonce?: string | null;
    actor?: (Entity & IsActor) | null;
    graph: Graph;
    epoch?: number;
  }) {
    this.oracle = options?.oracle ?? WORLD_ORACLE;
    this.client = options?.client ?? null;
    this.clientNonce = options?.clientNonce ?? null;
    this.actor = options?.actor ?? null;
    this.graph = options.graph;
    this.pendingEvents = [];
    this.connections = [];
    this.closedAt = null;
    this._epoch = options?.epoch ?? null;
    this._token = null;
  }

  repr(): string {
    const contentParts: string[] = [];
    if (this.actor) {
      contentParts.push(`actor=${this.actor.repr()}`);
    }
    if (this.graph) {
      contentParts.push(`graph=${this.graph.repr()}`);
    }
    if (this.closedAt) {
      contentParts.push(`closed_at=${this.closedAt.toString()}`);
    }
    return `<${this.constructor.name} ${contentParts.join(", ")}>`;
  }

  get epoch(): number {
    if (this._epoch == null) {
      throw new Error(`${this.repr()} has no epoch`);
    }
    return this._epoch;
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

  /** Append an Event. */
  append(event: Event): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this.pendingEvents.push(event);
  }

  /** Create a new Entity. */
  create(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({
      type: EditType.CREATE,
      node,
      value: toValue(node, null, { nodeAsValue: true }),
      space: node.spacePtr,
      branch: node.branchPtr,
      snapshot: node.snapshotPtr,
    });
    this.pendingEvents.push(edit);
    node._isNew = false;
  }

  /** Create or update an Entity. */
  upsert(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({
      type: EditType.UPSERT,
      node,
      value: toValue(node, null, { nodeAsValue: true }),
      space: node.spacePtr,
      branch: node.branchPtr,
      snapshot: node.snapshotPtr,
    });
    this.pendingEvents.push(edit);
    node._isNew = false;
  }

  /** Update an Entity. */
  update(node: Entity, edit: EditEvent): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this.pendingEvents.push(edit);
  }

  /** Set a Property on an Entity (direct SET/CLEAR operations). */
  updateSetProperty(node: Entity, prop: PropertyDefinition, newValue: any): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const propName = toCasing(prop.name, Casing.CAMEL);
    const nodePtr = node.toRef();
    const propType = prop.toType();

    // undo
    const undoOperation: EditOperation = EditOperation.SET;
    let oldValue: Value | null = (node as any)[propName];
    oldValue = toValue(oldValue, propType);

    // do
    const operation: EditOperation = EditOperation.SET;
    newValue = toValue(newValue, propType);

    // edit
    const edit = new EditEvent({
      type: EditType.UPDATE,
      node: nodePtr,
      propertyId: prop.id,
      operation: operation,
      value: newValue,
      reverseOperation: undoOperation,
      reverseValue: oldValue,
      space: node.spacePtr,
      branch: node.branchPtr,
      snapshot: node.snapshotPtr,
    });
    this.update(node, edit);
  }

  /** Move a Node to a new parent. */
  move(node: Entity, parent: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({
      type: EditType.MOVE,
      node,
      value: toValue(parent),
      space: node.spacePtr,
      branch: node.branchPtr,
      snapshot: node.snapshotPtr,
    });
    this.pendingEvents.push(edit);
  }

  /** Delete an Entity. */
  delete(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({
      type: EditType.DELETE,
      node,
      value: toValue(node, null, { nodeAsValue: true }),
      space: node.spacePtr,
      branch: node.branchPtr,
      snapshot: node.snapshotPtr,
    });
    this.pendingEvents.push(edit);
  }

  /** Restore a deleted Entity. */
  restore(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({
      type: EditType.RESTORE,
      node,
      space: node.spacePtr,
      branch: node.branchPtr,
      snapshot: node.snapshotPtr,
    });
    this.pendingEvents.push(edit);
  }

  _onFlush(): void {
    // nothing to do
  }

  /** Stage pending Edits and Changes. */
  async flush(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._onFlush();
  }

  /** Commit all Events. */
  async commit(): Promise<Event[]> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._onFlush();
    const events = this.pendingEvents;
    this.pendingEvents = [];
    throw new Error("not implemented");
  }
}
