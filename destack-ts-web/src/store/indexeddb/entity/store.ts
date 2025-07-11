import { IndexedDBStoreBase } from "@destack-web/store/indexeddb/core";
import { EditEvent, EntityStore, Query, QueryResult, StoreImplementation } from "@destack/language";
import { IDBPTransaction } from "idb";

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

  override async open(): Promise<void> {
    await super.open();
    if (this.db == null) {
      throw new Error(`database is not open in ${this.repr()}`);
    }

    // fetch initial entity count
    let entityCount = 0;
    for (const entityTable of this.schema.entityTables.values()) {
      const count = await this.db.count(entityTable.name);
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
