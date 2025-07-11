import { getNodeTypesForStores, Node, NodeReference, NodeType, StoreKey } from "@destack/language";

export const MAX_RECURSION_DEPTH = 100;

export const NODE_PARENT_KEY = String(Node.property("parent").id);
export const NODE_ID_ID = Node.property("id").id;
export const NODE_ID_KEY = String(Node.property("id").id);

export const NODE_REFERENCE_TYPE_KEY = String(NodeReference.property("type").id);
export const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);
export const NODE_REFERENCE_SPACE_ID_KEY = String(NodeReference.property("space_id").id);
export const NODE_REFERENCE_DEFINITION_ID_KEY = String(NodeReference.property("definition_id").id);

/** Base class for all IndexedDB stores. */
export abstract class IndexedDBStoreBase {
  types: StoreKey[];
  nodeTypes: NodeType[];

  readonly dbIsBorrowed: boolean;
  readonly dbName: string;
  db: IDBDatabase | null;

  constructor(options: {
    types: StoreKey[];
    dbIsBorrowed: boolean;
    dbName?: string;
    db?: IDBDatabase;
  }) {
    this.types = options.types;
    this.nodeTypes = getNodeTypesForStores(this.types);
    this.dbIsBorrowed = options.dbIsBorrowed;
    this.dbName = options.dbName ?? "destack";
    this.db = options.db ?? null;
  }

  /** Migrate the database schema to the latest version. */
  abstract migrateSchema(db: IDBDatabase): void;

  /** Initialize the database connection. */
  async open(): Promise<void> {
    if (!this.dbIsBorrowed) {
      if (this.db != null) {
        throw new Error(`${this.dbName} database is already open`);
      }

      const promise = new Promise<IDBDatabase>((resolve, reject) => {
        const request = indexedDB.open(this.dbName, 1);
        request.onerror = (event) => {
          reject(new Error(`error opening database: ${event.target}`));
        };
        request.onsuccess = (event) => {
          const db = (event.target as IDBOpenDBRequest).result;
          resolve(db);
        };
      });
      this.db = await promise;
      this.migrateSchema(this.db);
    } else {
      if (this.db == null) {
        throw new Error(`${this.dbName} database is not open`);
      }
    }
  }

  /** Close the database connection. */
  async close(): Promise<void> {
    if (!this.dbIsBorrowed) {
      if (this.db == null) {
        throw new Error(`${this.dbName} database is not open`);
      }
      this.db.close();
      this.db = null;
    }
  }
}
