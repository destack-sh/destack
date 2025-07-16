import {
  EditEvent,
  EntityStore,
  Event,
  EventStore,
  getNodeTypesForStores,
  NodeType,
  Query,
  QueryResult,
  StoreDomain,
  StoreKey,
} from "@destack/language";
import { MemoryContext, MemoryDatabase } from "@destack/store/memory/core";
import { MemoryEntityStore } from "@destack/store/memory/entity/store";
import { MemoryEventStore } from "@destack/store/memory/event/store";
import { assertNever } from "@destack/utils";

/** A combined in-memory Store for Events and Entities. */
export class MemoryStore implements EventStore, EntityStore {
  public keys: StoreKey[];
  public nodeTypes: NodeType[];
  public database: MemoryDatabase;
  public context: MemoryContext;

  public eventStore: MemoryEventStore;
  public entityStore: MemoryEntityStore;

  constructor(options: { keys: StoreKey[]; database?: MemoryDatabase }) {
    this.keys = options.keys;
    this.nodeTypes = getNodeTypesForStores(this.keys);
    this.database = options.database ?? new MemoryDatabase();
    this.context = new MemoryContext(this.database);

    this.eventStore = new MemoryEventStore({ keys: this.keys, database: this.database });
    this.entityStore = new MemoryEntityStore({ keys: this.keys, database: this.database });
  }

  toString(): string {
    return `entities=${this.entityStore.repr()}, events=${this.eventStore.repr()}`;
  }

  repr(): string {
    return `<MemoryStore ${this.toString()}>`;
  }

  async open(): Promise<void> {
    await this.eventStore.open();
    await this.entityStore.open();
  }

  async close(): Promise<void> {
    await this.eventStore.close();
    await this.entityStore.close();
  }

  query(query: Query): Promise<QueryResult> {
    if (query.domain === StoreDomain.ENTITY) {
      return this.entityStore.query(query);
    } else if (query.domain === StoreDomain.EVENT) {
      return this.eventStore.query(query);
    } else {
      assertNever(query.domain);
    }
  }

  async commit(events: EditEvent[]): Promise<EditEvent[]> {
    const appliedEvents = await this.append(events);
    return appliedEvents as EditEvent[];
  }

  async append(events: Event[]): Promise<Event[]> {
    await this.eventStore.append(events);
    const editEvents = events.filter((event) => event instanceof EditEvent);
    await this.entityStore.commit(editEvents);
    return events;
  }
}
