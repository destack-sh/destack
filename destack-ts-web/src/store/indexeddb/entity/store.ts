import { IndexedDBStoreBase } from "@destack-web/store/indexeddb/core";
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

  toString(): string {
    // nocheckin: implement proper string representation
    return `entities=0`;
  }

  repr(): string {
    return `<IndexedDBEntityStore ${this.toString()}>`;
  }

  override migrateSchema(db: IDBDatabase): void {
    throw new Error("Not implemented");
  }

  async query(query: Query): Promise<QueryResult> {
    throw new Error("Not implemented");
  }

  async commit(events: EditEvent[]): Promise<EditEvent[]> {
    throw new Error("Not implemented");
  }
}

/** Get the object store name for an Entity type. */
export function getEntityStoreName(nodeType: NodeType): string {
  return `destack_${nodeType}`;
}

/** Convert Entity id and snapshot id to a composite key. */
export function makeEntityKey(id: string, snapshotId: string | null): string {
  return `${id}:${snapshotId || "<root>"}`;
}

/** Parse a composite Entity key. */
export function parseEntityKey(key: string): { id: string; snapshotId: string | null } {
  const [id, snapshot] = key.split(":");
  return {
    id,
    snapshotId: snapshot === "<root>" ? null : snapshot,
  };
}
