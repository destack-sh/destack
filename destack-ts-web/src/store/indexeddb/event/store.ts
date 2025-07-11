import { IndexedDBStoreBase } from "@destack-web/store/indexeddb/core";
import {
  Event,
  EventStore,
  NodeType,
  Query,
  QueryResult,
  StoreImplementation,
} from "@destack/language";

/** An IndexedDB Store for Events. */
export class IndexedDBEventStore extends IndexedDBStoreBase implements EventStore {
  public static readonly implementation: StoreImplementation = StoreImplementation.INDEXEDDB;

  toString(): string {
    // nocheckin: implement proper string representation
    return `events=0`;
  }

  repr(): string {
    return `<IndexedDBEventStore ${this.toString()}>`;
  }

  override migrateSchema(db: IDBDatabase): void {
    throw new Error("Not implemented");
  }

  async query(query: Query): Promise<QueryResult> {
    throw new Error("Not implemented");
  }

  async append(events: Event[]): Promise<Event[]> {
    throw new Error("Not implemented");
  }
}

/** Get the object store name for an event type. */
export function getEventStoreName(nodeType: NodeType): string {
  return `destack_event`; // combine all in one
}
