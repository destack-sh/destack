import {
  EntityStore,
  EventStatus,
  EventStore,
  type Client,
  type Entity,
  type Event,
  type IsSubject,
  type Node,
  type PropertyDefinition,
  type QueryConnection,
  type Space,
} from "@destack/language";
import { TypeCardinality } from "@destack/language/core/builtin/common";
import { ACTIVE_SESSION } from "@destack/language/core/builtin/const";
import { EditEvent, EditOperation, EditType } from "@destack/language/core/builtin/edit";
import { toValue, Value } from "@destack/language/core/common/value";
import { Supergraph } from "@destack/language/core/runtime/graph";
import { WORLD_ORACLE, type Oracle } from "@destack/language/core/runtime/oracle";
import { assertNever, Casing, toCasing } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/**
 * A managed Session for interacting with Destack.
 */
export class Session {
  oracle: Oracle;
  space: Space | null;
  client: Client | null;
  clientNonce: string | null;
  subject: (Node & IsSubject) | null;
  store: EventStore | EntityStore | null;
  supergraph: Supergraph;

  pendingEvents: Event[];

  connections: QueryConnection[];
  closedAt: Temporal.ZonedDateTime | null;
  _token: string | null;

  constructor(options?: {
    oracle?: Oracle;
    space?: Space | null;
    client?: Client | null;
    clientNonce?: string | null;
    subject?: (Node & IsSubject) | null;
    store?: EventStore | EntityStore | null;
    supergraphFactory?: (session: Session) => Supergraph;
  }) {
    this.oracle = options?.oracle ?? WORLD_ORACLE;
    this.space = options?.space ?? null;
    this.client = options?.client ?? null;
    this.clientNonce = options?.clientNonce ?? null;
    this.subject = options?.subject ?? null;
    this.store = options?.store ?? null;
    this.supergraph =
      options?.supergraphFactory != null ? options.supergraphFactory(this) : new Supergraph(this);

    // runtime
    this.pendingEvents = [];
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

    // edit
    const edit = new EditEvent({
      type: EditType.UPDATE,
      node: nodePtr,
      attribute: propPtr,
      operation: operation,
      value: newValue,
      reverseOperation: undoOperation,
      reverseValue: oldValue,
    });
    this.update(node, edit);
  }

  /** Move a Node to a new parent. */
  move(node: Entity, parent: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({ type: EditType.MOVE, node, value: toValue(parent) });
    this.pendingEvents.push(edit);
  }

  /** Archive an Entity. */
  archive(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({
      type: EditType.ARCHIVE,
      node,
      value: toValue(node, null, { nodeAsValue: true }),
    });
    this.pendingEvents.push(edit);
  }

  /** Unarchive an Entity. */
  unarchive(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({ type: EditType.UNARCHIVE, node });
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
    });
    this.pendingEvents.push(edit);
  }

  /** Restore a deleted Entity. */
  restore(node: Entity): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    const edit = new EditEvent({ type: EditType.RESTORE, node });
    this.pendingEvents.push(edit);
  }

  /** Flush pending Edits and Changes. */
  flush(): void {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    // nothing to do?
  }

  /** Stage pending Edits and Changes. */
  async stage(): Promise<void> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    }
    this.flush();
  }

  /** Commit all Events. */
  async commit(): Promise<Event[]> {
    if (this.closedAt) {
      throw new Error(`${this.repr()} is closed`);
    } else if (this.store == null) {
      throw new Error(`${this.repr()} has no Store`);
    }
    this.flush();
    const events = this.pendingEvents;
    this.pendingEvents = [];

    // commit
    let appliedEvents: Event[];
    if (this.store == null) {
      throw new Error(`${this.repr()} has no Store`);
    } else if ("append" in this.store) {
      appliedEvents = await this.store.append(events);
    } else if ("commit" in this.store) {
      const editEvents = events.filter((event) => event instanceof EditEvent);
      appliedEvents = await this.store.commit(editEvents);
    } else {
      assertNever(this.store);
    }

    // check
    if (appliedEvents.some((event) => event.status != EventStatus.APPROVED)) {
      const badEvents = appliedEvents.filter((event) => event.status != EventStatus.APPROVED);
      // TODO :Incomplete: do something on :RejectedEvents
      // throw new Error(
      //   `failed to commit ${events.length} Events: ${badEvents
      //     .map((event) => event.id)
      //     .join(", ")}`,
      // );
    }
    return appliedEvents;
  }
}
