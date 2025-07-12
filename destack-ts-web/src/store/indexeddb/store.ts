import { IndexedDBStoreBase } from "@destack-web/store/indexeddb/core";
import { IndexedDBEntityStore } from "@destack-web/store/indexeddb/entity/store";
import { IndexedDBEventStore } from "@destack-web/store/indexeddb/event/store";
import { getIndexedDBSchema } from "@destack-web/store/indexeddb/map";
import {
  EditEvent,
  EntityStore,
  Event,
  EventStore,
  Query,
  QueryResult,
  StoreDomain,
  StoreImplementation,
  StoreKey,
} from "@destack/language";
import { assertNever } from "@destack/utils";

/** A combined IndexedDB Store for Events and Entities. */
export class IndexedDBStore extends IndexedDBStoreBase implements EventStore, EntityStore {
  public static readonly implementation: StoreImplementation = StoreImplementation.INDEXEDDB;

  public entityStore: IndexedDBEntityStore;
  public eventStore: IndexedDBEventStore;

  constructor(options: { types: StoreKey[]; dbName?: string }) {
    super({
      ...options,
      schema: getIndexedDBSchema(),
      dbIsBorrowed: false,
    });

    this.entityStore = new IndexedDBEntityStore({
      types: this.types,
      schema: this.schema,
      context: this.context,
      dbIsBorrowed: true,
      dbName: this.dbName,
    });
    this.eventStore = new IndexedDBEventStore({
      types: this.types,
      schema: this.schema,
      context: this.context,
      dbIsBorrowed: true,
      dbName: this.dbName,
    });
  }

  override async open(): Promise<void> {
    await super.open();
    if (this.db == null) {
      throw new Error("database is not open");
    }
    this.entityStore.db = this.db;
    this.eventStore.db = this.db;

    await this.entityStore.open();
    await this.eventStore.open();
  }

  toString(): string {
    return `entities=${this.entityStore.repr()}, events=${this.eventStore.repr()}`;
  }

  repr(): string {
    return `<IndexedDBStore ${this.toString()}>`;
  }

  async query(query: Query): Promise<QueryResult> {
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
    if (this.db == null) {
      throw new Error(`database is not open in ${this.repr()}`);
    }
    const tx = this.db.transaction(this.schema.tableNames, "readwrite");
    await this.eventStore.append(events, { tx });
    const editEvents = events.filter((event) => event instanceof EditEvent);
    await this.entityStore.commit(editEvents, { tx });
    await tx.done;
    return events;
  }
}
