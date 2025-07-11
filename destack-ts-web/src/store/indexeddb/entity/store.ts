import { ALIAS_PREFIX, IndexedDBStoreBase, NODE_ID_KEY } from "@destack-web/store/indexeddb/core";
import {
  EditEvent,
  EntityStore,
  NodeType,
  Query,
  QueryResult,
  StoreImplementation,
} from "@destack/language";

/** An IndexedDB Store for Entities. */
export class IndexedDBEntityStore extends IndexedDBStoreBase implements EntityStore {
  public static readonly implementation: StoreImplementation = StoreImplementation.INDEXEDDB;

  private entityCount: number | null = null;

  toString(): string {
    return `entities=${this.entityCount ?? "<unknown>"}`;
  }

  repr(): string {
    return `<IndexedDBEntityStore ${this.toString()}>`;
  }

  override migrateSchema(db: IDBDatabase): void {
    // entity stores
    let entityStore: IDBObjectStore;
    for (const nodeType of this.nodeTypes) {
      const storeName = getEntityStoreName(nodeType);
      if (!db.objectStoreNames.contains(storeName)) {
        entityStore = db.createObjectStore(storeName, { keyPath: ALIAS_PREFIX + NODE_ID_KEY });
      }
    }
  }

  override async open(): Promise<void> {
    await super.open();
    if (this.db == null) {
      throw new Error(`database is not open in ${this.repr()}`);
    }

    // fetch initial entity count
    let entityCount = 0;
    for (const nodeType of this.nodeTypes) {
      const storeName = getEntityStoreName(nodeType);
      const request = this.db.transaction(storeName, "readonly").objectStore(storeName).count();
      request.onsuccess = (event) => {
        entityCount += (event.target as IDBRequest).result;
      };
    }
    this.entityCount = entityCount;
  }

  async query(
    query: Query,
    options?: {
      tx?: IDBTransaction;
    },
  ): Promise<QueryResult> {
    throw new Error("Not implemented");
  }

  async commit(
    events: EditEvent[],
    options?: {
      tx?: IDBTransaction;
    },
  ): Promise<EditEvent[]> {
    throw new Error("Not implemented");
  }
}

/** Get the object store name for an Entity type. */
export function getEntityStoreName(nodeType: NodeType): string {
  return `destack_${nodeType}`;
}
