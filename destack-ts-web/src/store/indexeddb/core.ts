import { getIndexName } from "@destack-web/store/indexeddb/map";
import {
  Entity,
  Event,
  getNodeTypesForStores,
  getSubdefinitionsForNodeType,
  IsExtensible,
  Node,
  NodeDefinitionReference,
  NodeReference,
  NodeType,
  StoreKey,
  VERSION,
} from "@destack/language";
import { assertNever } from "@destack/utils";
import { intToSemver, semverToInt } from "@destack/utils/semver";
import { IDBPDatabase, IDBPTransaction, openDB } from "idb";

export const MAX_RECURSION_DEPTH = 100;

export const NULL_SENTINEL = "__NULL__";

export const NODE_ID_ID = Node.property("id").id;
export const NODE_ID_KEY = String(Node.property("id").id);
export const NODE_METATYPE_KEY = String(Node.property("metatype").id);
export const NODE_SPACE_PTR_ID = String(Node.property("space").id);
export const NODE_DEFINITION_PTR_ID = String(IsExtensible.property("definition").id);

export const NODE_REFERENCE_TYPE_KEY = String(NodeReference.property("type").id);
export const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);
export const NODE_REFERENCE_SPACE_ID_KEY = String(NodeReference.property("space_id").id);
export const NODE_REFERENCE_DEFINITION_ID_KEY = String(NodeReference.property("definition_id").id);

export const ENTITY_PARENT_KEY = String(Entity.property("parent").id);
export const ENTITY_SNAPSHOT_KEY = String(Entity.property("snapshot").id);
export const ENTITY_MATERIALIZATION_KEY = String(Entity.property("materialization").id);
export const ENTITY_CREATED_AT_KEY = String(Entity.property("created_at").id);

export const EVENT_CREATED_AT_KEY = String(Event.property("created_at").id);
export const EVENT_SNAPSHOT_KEY = String(Event.property("snapshot").id);

// NOTE: IndexedDB doesn't allow numeric keys for indexes (incl. primary keys)
//  (so even simple keys like id and created_at are prefixed)
export const ENTITY_PRIMARY_KEY = "_0"; // composite key of [id, snapshotId]
export const ENTITY_KEYS_TO_INDEX: string[] = [
  ENTITY_PRIMARY_KEY,
  ENTITY_PARENT_KEY,
  ENTITY_SNAPSHOT_KEY,
];
export const ENTITY_KEYS_TO_INDEX_PREFIXED: Record<string, string> = ENTITY_KEYS_TO_INDEX.reduce(
  (acc, key) => {
    acc[key] = key.startsWith("_") ? key : "_" + key;
    return acc;
  },
  {} as Record<string, string>,
);
export const EVENT_KEYS_TO_INDEX: string[] = [
  NODE_METATYPE_KEY, // events may be shared across tables
  NODE_ID_KEY,
  EVENT_CREATED_AT_KEY,
  EVENT_SNAPSHOT_KEY,
];
export const EVENT_KEYS_TO_INDEX_PREFIXED: Record<string, string> = EVENT_KEYS_TO_INDEX.reduce(
  (acc, key) => {
    acc[key] = key.startsWith("_") ? key : "_" + key;
    return acc;
  },
  {} as Record<string, string>,
);

/** Base class for all IndexedDB stores. */
export abstract class IndexedDBStoreBase {
  keys: StoreKey[];
  nodeTypes: NodeType[];
  schema: IndexedDBSchema;
  context: IndexedDBContext;

  readonly dbIsBorrowed: boolean;
  readonly dbName: string;
  db: IDBPDatabase | null;

  constructor(options: {
    keys: StoreKey[];
    schema: IndexedDBSchema;
    context?: IndexedDBContext;
    dbIsBorrowed: boolean;
    dbName?: string;
    db?: IDBPDatabase;
  }) {
    this.keys = options.keys;
    this.nodeTypes = getNodeTypesForStores(this.keys);
    this.schema = options.schema;
    this.context = options.context ?? new IndexedDBContext(this.schema);

    this.dbIsBorrowed = options.dbIsBorrowed;
    this.dbName = options.dbName ?? "destack";
    this.db = options.db ?? null;
  }

  /** Migrate the database schema to the latest version. */
  migrateSchema(
    db: IDBPDatabase,
    oldVersion: string | null,
    newVersion: string,
    tx: IDBPTransaction<unknown, string[], "versionchange">,
  ): void {
    for (const table of [
      ...this.schema.eventTables.values(),
      ...this.schema.entityTables.values(),
    ]) {
      let tableStore;
      if (!db.objectStoreNames.contains(table.name)) {
        if (table instanceof IndexedDBEntityTable) {
          tableStore = db.createObjectStore(table.name, {
            keyPath: ENTITY_KEYS_TO_INDEX_PREFIXED[ENTITY_PRIMARY_KEY],
          });
        } else if (table instanceof IndexedDBEventTable) {
          tableStore = db.createObjectStore(table.name, {
            keyPath: EVENT_KEYS_TO_INDEX_PREFIXED[NODE_ID_KEY],
          });
        } else {
          assertNever(table);
        }
      } else {
        tableStore = tx.objectStore(table.name);
      }
      // index
      const objectStore = tx.objectStore(table.name);
      for (const indexedProp of table.indexedKeys) {
        const indexName = getIndexName(table, indexedProp);
        if (!objectStore.indexNames.contains(indexName)) {
          if (table instanceof IndexedDBEntityTable) {
            tableStore.createIndex(indexName, ENTITY_KEYS_TO_INDEX_PREFIXED[indexedProp]);
          } else if (table instanceof IndexedDBEventTable) {
            tableStore.createIndex(indexName, EVENT_KEYS_TO_INDEX_PREFIXED[indexedProp]);
          } else {
            assertNever(table);
          }
        }
      }
    }
  }

  /** Initialize the database connection. */
  async open(): Promise<void> {
    if (!this.dbIsBorrowed) {
      if (this.db != null) {
        throw new Error(`${this.dbName} database is already open`);
      }
      const newVersion = VERSION;
      const newVersionInt = semverToInt(newVersion);
      this.db = await openDB(this.dbName, newVersionInt, {
        upgrade: (db, oldVersionInt, newVersionInt, tx) => {
          const oldVersion = oldVersionInt == null ? null : intToSemver(oldVersionInt);
          this.migrateSchema(db, oldVersion, newVersion, tx);
        },
      });
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

/** The schema of an IndexedDB database. */
export class IndexedDBSchema {
  readonly entityTables: Readonly<Map<NodeType, IndexedDBEntityTable>>;
  readonly eventTables: Readonly<Map<NodeType, IndexedDBEventTable>>;
  readonly tableNames: readonly string[];

  constructor(options: {
    entityTables: Map<NodeType, IndexedDBEntityTable>;
    eventTables: Map<NodeType, IndexedDBEventTable>;
  }) {
    this.entityTables = options.entityTables;
    this.eventTables = options.eventTables;
    this.tableNames = [
      ...this.entityTables.values().map((table) => table.name),
      ...this.eventTables.values().map((table) => table.name),
    ];
  }

  toString(): string {
    return `entityTables=${this.entityTables.size}, eventTables=${this.eventTables.size}`;
  }

  repr(): string {
    return `<${this.constructor.name} ${this.toString()}>`;
  }
}

/** An Entity table in an IndexedDB database. */
export class IndexedDBEntityTable {
  nodeType: NodeType;
  name: string;
  indexedKeys: string[];

  constructor(options: { nodeType: NodeType; name: string; indexedKeys: string[] }) {
    this.nodeType = options.nodeType;
    this.name = options.name;
    this.indexedKeys = options.indexedKeys;
  }

  toString(): string {
    return `nodeType=${NodeType[this.nodeType]}, name=${this.name}`;
  }

  repr(): string {
    return `<${this.constructor.name} ${this.toString()}>`;
  }
}

/** An Event table in an IndexedDB database. */
export class IndexedDBEventTable {
  nodeType: NodeType;
  name: string;
  indexedKeys: string[];

  constructor(options: { nodeType: NodeType; name: string; indexedKeys: string[] }) {
    this.nodeType = options.nodeType;
    this.name = options.name;
    this.indexedKeys = options.indexedKeys;
  }

  toString(): string {
    return `nodeType=${NodeType[this.nodeType]}, name=${this.name}`;
  }

  repr(): string {
    return `<${this.constructor.name} ${this.toString()}>`;
  }
}

/** The current context for working with an IndexedDB database. */
export class IndexedDBContext {
  schema: IndexedDBSchema;

  constructor(schema: IndexedDBSchema) {
    this.schema = schema;
  }

  toString(): string {
    return `schema=${this.schema.toString()}`;
  }

  repr(): string {
    return `<IndexedDBContext ${this.toString()}>`;
  }

  /** Expand the (separately) stored definitions for a NodeDefinition. */
  resolve(definition: NodeDefinitionReference): NodeDefinitionReference[] {
    return getSubdefinitionsForNodeType(definition.nodeType);
  }

  /** Get the entity table for a NodeDefinition. */
  getEntityTable(definition: NodeDefinitionReference | NodeReference): IndexedDBEntityTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    if (!this.schema.entityTables.has(nodeType)) {
      throw new Error(`entity table for ${nodeType} not found in ${this.repr()}`);
    }
    return this.schema.entityTables.get(nodeType)!;
  }

  /** Get the event table for a NodeDefinition. */
  getEventTable(definition: NodeDefinitionReference | NodeReference): IndexedDBEventTable {
    const eventTable = this.schema.eventTables.get(NodeType.EVENT);
    if (eventTable == null) {
      throw new Error(`event table for ${NodeType.EVENT} not found in ${this.repr()}`);
    }
    return eventTable;
  }
}
