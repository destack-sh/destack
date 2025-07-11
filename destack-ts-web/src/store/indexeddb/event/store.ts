import {
  ALIAS_PREFIX,
  EVENT_CREATED_AT_KEY,
  IndexedDBStoreBase,
  NODE_ID_KEY,
  NODE_METATYPE_KEY,
} from "@destack-web/store/indexeddb/core";
import { Event, EventStore, Query, QueryResult, StoreImplementation } from "@destack/language";
import { IDBPDatabase, IDBPObjectStore, IDBPTransaction } from "idb";

export const DESTACK_EVENT_STORE_NAME = "destack_event";

/** An IndexedDB Store for Events. */
export class IndexedDBEventStore extends IndexedDBStoreBase implements EventStore {
  public static readonly implementation: StoreImplementation = StoreImplementation.INDEXEDDB;

  private eventCount: number | null = null;

  toString(): string {
    return `events=${this.eventCount ?? "<unknown>"}`;
  }

  repr(): string {
    return `<IndexedDBEventStore ${this.toString()}>`;
  }

  override migrateSchema(
    db: IDBPDatabase,
    oldVersion: string | null,
    newVersion: string,
    tx: IDBPTransaction<unknown, string[], "versionchange">,
  ): void {
    // event store
    let eventStore;
    if (!db.objectStoreNames.contains(DESTACK_EVENT_STORE_NAME)) {
      eventStore = db.createObjectStore(DESTACK_EVENT_STORE_NAME, {
        keyPath: ALIAS_PREFIX + NODE_ID_KEY,
      });
    } else {
      eventStore = tx.objectStore(DESTACK_EVENT_STORE_NAME);
    }
    // index
    for (const indexedProp of [NODE_METATYPE_KEY, EVENT_CREATED_AT_KEY]) {
      const indexName = `${DESTACK_EVENT_STORE_NAME}_${indexedProp}`;
      if (!eventStore.indexNames.contains(indexName)) {
        eventStore.createIndex(indexName, ALIAS_PREFIX + indexedProp);
      }
    }
  }

  override async open(): Promise<void> {
    await super.open();
    if (this.db == null) {
      throw new Error(`database is not open in ${this.repr()}`);
    }

    // fetch initial event count
    const eventCount = await this.db.count(DESTACK_EVENT_STORE_NAME);
    this.eventCount = eventCount;
  }

  async query(
    query: Query,
    options?: {
      tx?: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
    },
  ): Promise<QueryResult> {
    throw new Error("Not implemented");
  }

  async append(
    events: Event[],
    options?: {
      tx?: IDBPTransaction<unknown, string[], "readwrite">;
    },
  ): Promise<Event[]> {
    if (this.eventCount == null) {
      throw new Error(`event count not initialized in ${this.repr()}`);
    }
    this.eventCount += events.length;
    throw new Error("Not implemented");
  }
}
