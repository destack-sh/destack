import { IndexedDBStoreBase } from "@destack-web/store/indexeddb/core";
import { executeEdits } from "@destack-web/store/indexeddb/entity/edit";
import {
  EditEvent,
  EditType,
  EntityStore,
  Query,
  QueryResult,
  StoreImplementation,
} from "@destack/language";
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
    if (this.db == null) {
      throw new Error(`database is not open in ${this.repr()}`);
    } else if (events.length === 0) {
      return [];
    }

    let tx: IDBPTransaction<unknown, string[], "readwrite">;
    let shouldCommit = false;

    if (options?.tx) {
      // use the provided transaction
      tx = options.tx;
    } else {
      // create a new transaction with all required table names
      const tableNames = new Set<string>();
      for (const edit of events) {
        const table = this.context.getEntityTable(edit.nodePtr);
        tableNames.add(table.name);
      }
      tx = this.db.transaction(Array.from(tableNames), "readwrite");
      shouldCommit = true;
    }

    const { edits, cascadedEdits } = await executeEdits({
      tx,
      context: this.context,
      edits: events,
    });

    // commit the transaction if we created it
    if (shouldCommit) {
      await tx.done;
    }

    // update entity count
    // NOTE :Robustness: IndexedDBEntityStore.entityCount is approximate
    if (this.entityCount !== null) {
      for (const edit of [...edits, ...cascadedEdits]) {
        if (edit.type === EditType.CREATE || edit.type === EditType.UPSERT) {
          this.entityCount++;
        } else if (edit.type === EditType.ERASE) {
          this.entityCount--;
        }
      }
    }

    return [...edits, ...cascadedEdits];
  }
}
