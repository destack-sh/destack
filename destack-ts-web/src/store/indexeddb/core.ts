import {
  Entity,
  Event,
  getNodeTypesForStores,
  IsExtensible,
  IsSpatial,
  Node,
  NodeReference,
  NodeType,
  StoreKey,
} from "@destack/language";

export const MAX_RECURSION_DEPTH = 100;


export const NODE_PARENT_KEY = String(Node.property("parent").id);
export const NODE_ID_ID = Node.property("id").id;
export const NODE_ID_KEY = String(Node.property("id").id);
export const NODE_METATYPE_KEY = String(Node.property("metatype").id);
export const NODE_PARENT_PTR_KEY = String(Node.property("parent").id);
export const NODE_SPACE_PTR_ID = String(IsSpatial.property("space").id);
export const NODE_DEFINITION_PTR_ID = String(IsExtensible.property("definition").id);

export const NODE_REFERENCE_TYPE_KEY = String(NodeReference.property("type").id);
export const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);
export const NODE_REFERENCE_SPACE_ID_KEY = String(NodeReference.property("space_id").id);
export const NODE_REFERENCE_DEFINITION_ID_KEY = String(NodeReference.property("definition_id").id);

export const ENTITY_SNAPSHOT_PTR_KEY = String(Entity.property("snapshot").id);
export const ENTITY_MATERIALIZATION_KEY = String(Entity.property("materialization").id);
export const ENTITY_CREATED_AT_KEY = String(Entity.property("created_at").id);
export const ENTITY_ALIASED_KEYS = [NODE_METATYPE_KEY, NODE_ID_KEY, ENTITY_CREATED_AT_KEY];

export const EVENT_CREATED_AT_KEY = String(Event.property("created_at").id);
export const EVENT_SNAPSHOT_PTR_KEY = String(Event.property("snapshot").id);
export const EVENT_ALIASED_KEYS = [NODE_METATYPE_KEY, NODE_ID_KEY, EVENT_CREATED_AT_KEY];

export const ALIAS_PREFIX = "_";

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
        request.onupgradeneeded = (event) => {
          const db = (event.target as IDBOpenDBRequest).result;
          this.migrateSchema(db);
        };
        request.onsuccess = (event) => {
          const db = (event.target as IDBOpenDBRequest).result;
          resolve(db);
        };
      });
      this.db = await promise;
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
