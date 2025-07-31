import type {
  Client,
  Entity,
  Event,
  GraphConnection,
  NodeReference,
  PropertyDefinition,
} from "@destack/language";
import { ACTIVE_SESSION } from "@destack/language/core/builtin/const";
import { EditEvent, EditOperation, EditType } from "@destack/language/core/builtin/edit";
import { toValue, type Value } from "@destack/language/core/builtin/value";
import type { Graph } from "@destack/language/core/runtime/graph";
import { type Oracle, WORLD_ORACLE } from "@destack/language/core/runtime/oracle";
import { Casing, toCasing } from "@destack/utils";
import type { Temporal } from "temporal-polyfill";

/**
 * A managed Session for interacting with Destack.
 */
export class Session {
  graph: Graph;
  remoteEpoch: number;
  localEpoch: number;

  clientPtr: NodeReference;
  clientNonce: string;
  actorPtr: NodeReference;
  oracle: Oracle;

  pendingEvents: Event[];
  connections: GraphConnection[];

  closedAt: Temporal.ZonedDateTime | null;
  _token: string | null;

  constructor(options: {
    graph: Graph;
    remoteEpoch: number;
    localEpoch: number;
    actor: Entity | NodeReference;
    client: Client | NodeReference;
    clientNonce: string;
    oracle?: Oracle;
  }) {
    this.oracle = options?.oracle ?? WORLD_ORACLE;
    this.clientPtr = options.client.toRef();
    this.clientNonce = options.clientNonce;
    this.actorPtr = options.actor.toRef();
    this.graph = options.graph;
    this.remoteEpoch = options.remoteEpoch;
    this.localEpoch = options.localEpoch;

    this.pendingEvents = [];
    this.connections = [];
    this.closedAt = null;
    this._token = null;
  }

  repr(): string {
    const contentParts: string[] = [];
    if (this.actorPtr) {
      contentParts.push(`actor=${this.actorPtr.repr()}`);
    }
    if (this.graph) {
      contentParts.push(`graph=${this.graph.repr()}`);
    }
    if (this.closedAt) {
      contentParts.push(`closed_at=${this.closedAt.toString()}`);
    }
    return `<${this.constructor.name} ${contentParts.join(", ")}>`;
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
    if (prop.name === null) {
      throw new Error(`property name is required for ${node.repr()}.${prop.id}`);
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
  async flush() {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._onFlush();
  }

  /** Commit all Events. */
  async commit() {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this._onFlush();
    const events = this.pendingEvents;
    this.pendingEvents = [];
    // nocheckin(all): Session.flush/commit
  }
}
