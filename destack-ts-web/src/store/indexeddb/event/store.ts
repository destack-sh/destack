import {
  ALIAS_PREFIX,
  EVENT_CREATED_AT_KEY,
  IndexedDBStoreBase,
  NODE_ID_KEY,
  NODE_METATYPE_KEY,
} from "@destack-web/store/indexeddb/core";
import { Event, EventStore, Query, QueryResult, StoreImplementation } from "@destack/language";

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

  override migrateSchema(db: IDBDatabase): void {
    // event store
    let eventStore: IDBObjectStore;
    if (!db.objectStoreNames.contains(DESTACK_EVENT_STORE_NAME)) {
      eventStore = db.createObjectStore(DESTACK_EVENT_STORE_NAME, {
        keyPath: ALIAS_PREFIX + NODE_ID_KEY,
      });
    } else {
      eventStore = db
        .transaction(DESTACK_EVENT_STORE_NAME, "readwrite")
        .objectStore(DESTACK_EVENT_STORE_NAME);
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
    let eventCount = 0;
    const request = this.db
      .transaction(DESTACK_EVENT_STORE_NAME, "readonly")
      .objectStore(DESTACK_EVENT_STORE_NAME)
      .count();
    request.onsuccess = (event) => {
      eventCount += (event.target as IDBRequest).result;
    };
    this.eventCount = eventCount;
  }

  async query(
    query: Query,
    options?: {
      tx?: IDBTransaction;
    },
  ): Promise<QueryResult> {
    throw new Error("Not implemented");
  }

  async append(
    events: Event[],
    options?: {
      tx?: IDBTransaction;
    },
  ): Promise<Event[]> {
    if (this.eventCount == null) {
      throw new Error(`event count not initialized in ${this.repr()}`);
    }
    this.eventCount += events.length;
    throw new Error("Not implemented");
  }
}
