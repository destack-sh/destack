import { ALIAS_PREFIX, IndexedDBStoreBase, NODE_ID_KEY } from "@destack-web/store/indexeddb/core";
import {
  EditEvent,
  EntityStore,
  NodeType,
  Query,
  QueryResult,
  StoreImplementation,
} from "@destack/language";
import { IDBPDatabase, IDBPTransaction } from "idb";

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

  override migrateSchema(
    db: IDBPDatabase,
    oldVersion: string | null,
    newVersion: string,
    tx: IDBPTransaction<unknown, string[], "versionchange">,
  ): void {
    // entity stores
    for (const nodeType of this.nodeTypes) {
      const storeName = getEntityStoreName(nodeType);
      if (!db.objectStoreNames.contains(storeName)) {
        db.createObjectStore(storeName, { keyPath: ALIAS_PREFIX + NODE_ID_KEY });
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
      const count = await this.db.count(storeName);
      entityCount += count;
    }
    this.entityCount = entityCount;
  }

  async query(
    query: Query,
    options?: {
      tx?: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
    },
  ): Promise<QueryResult> {
    throw new Error("Not implemented");
  }

  async commit(
    events: EditEvent[],
    options?: {
      tx?: IDBPTransaction<unknown, string[], "readwrite">;
    },
  ): Promise<EditEvent[]> {
    throw new Error("Not implemented");
  }
}

/** Get the object store name for an Entity type. */
export function getEntityStoreName(nodeType: NodeType): string {
  return `destack_${nodeType}`;
}
