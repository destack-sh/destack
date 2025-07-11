import {
  Entity,
  Event,
  getNodeTypesForStores,
  getSubdefinitionsForNodeType,
  IsExtensible,
  IsSpatial,
  Node,
  NodeDefinitionReference,
  NodeReference,
  NodeType,
  StoreKey,
} from "@destack/language";
import { intToSemver } from "@destack/utils/semver";
import { IDBPDatabase, IDBPTransaction, openDB } from "idb";

export const MAX_RECURSION_DEPTH = 100;

export const NODE_PARENT_KEY = String(Node.property("parent").id);
export const NODE_ID_ID = Node.property("id").id;
export const NODE_ID_KEY = String(Node.property("id").id);
export const NODE_METATYPE_KEY = String(Node.property("metatype").id);
export const NODE_SPACE_PTR_ID = String(IsSpatial.property("space").id);
export const NODE_DEFINITION_PTR_ID = String(IsExtensible.property("definition").id);

export const NODE_REFERENCE_TYPE_KEY = String(NodeReference.property("type").id);
export const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);
export const NODE_REFERENCE_SPACE_ID_KEY = String(NodeReference.property("space_id").id);
export const NODE_REFERENCE_DEFINITION_ID_KEY = String(NodeReference.property("definition_id").id);

export const ENTITY_SNAPSHOT_KEY = String(Entity.property("snapshot").id);
export const ENTITY_MATERIALIZATION_KEY = String(Entity.property("materialization").id);
export const ENTITY_CREATED_AT_KEY = String(Entity.property("created_at").id);
export const ENTITY_INDEXED_KEYS = [
  NODE_METATYPE_KEY,
  NODE_ID_KEY,
  NODE_PARENT_KEY,
  ENTITY_SNAPSHOT_KEY,
  ENTITY_CREATED_AT_KEY,
];

export const EVENT_CREATED_AT_KEY = String(Event.property("created_at").id);
export const EVENT_SNAPSHOT_KEY = String(Event.property("snapshot").id);
export const EVENT_INDEXED_KEYS = [
  NODE_METATYPE_KEY,
  NODE_ID_KEY,
  EVENT_SNAPSHOT_KEY,
  EVENT_CREATED_AT_KEY,
];

export const INDEXED_PREFIX = "_";

/** Base class for all IndexedDB stores. */
export abstract class IndexedDBStoreBase {
  types: StoreKey[];
  nodeTypes: NodeType[];
  schema: IndexedDBSchema;
  context: IndexedDBContext;

  readonly dbIsBorrowed: boolean;
  readonly dbName: string;
  db: IDBPDatabase | null;

  constructor(options: {
    types: StoreKey[];
    schema: IndexedDBSchema;
    context?: IndexedDBContext;
    dbIsBorrowed: boolean;
    dbName?: string;
    db?: IDBPDatabase;
  }) {
    this.types = options.types;
    this.nodeTypes = getNodeTypesForStores(this.types);
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
        tableStore = db.createObjectStore(table.name, { keyPath: INDEXED_PREFIX + NODE_ID_KEY });
      } else {
        tableStore = tx.objectStore(table.name);
      }
      // index
      for (const indexedProp of table.indexedKeys) {
        const indexName = `${table.name}_${indexedProp}`;
        if (!db.objectStoreNames.contains(indexName)) {
          tableStore.createIndex(indexName, INDEXED_PREFIX + indexedProp);
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
      this.db = await openDB(this.dbName, 1, {
        upgrade: (db, oldVersionInt, newVersionInt, tx) => {
          const oldVersion = oldVersionInt == null ? null : intToSemver(BigInt(oldVersionInt));
          const newVersion = intToSemver(BigInt(newVersionInt!));
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

  constructor(options: {
    entityTables: Map<NodeType, IndexedDBEntityTable>;
    eventTables: Map<NodeType, IndexedDBEventTable>;
  }) {
    this.entityTables = options.entityTables;
    this.eventTables = options.eventTables;
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
