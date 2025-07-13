import {
  Entity,
  Event,
  IsArchivable,
  IsDeletable,
  IsExtensible,
  Node,
  NodeDefinitionReference,
  NodeReference,
  NodeType,
  StoreDomain,
} from "@destack/language";
import { getSubdefinitionsForNodeType, NODE_CLASS_BY_TYPE } from "@destack/language/registry";
import { MemoryEntityTable } from "./entity/core";
import { MemoryEventTable } from "./event/core";

export const MAX_RECURSION_DEPTH = 100;

export const NODE_PARENT_KEY = String(Node.property("parent").id);
export const NODE_ID_ID = Node.property("id").id;
export const NODE_ID_KEY = String(Node.property("id").id);
export const NODE_METATYPE_KEY = String(Node.property("metatype").id);
export const NODE_SPACE_PTR_ID = String(Node.property("space").id);
export const NODE_DEFINITION_PTR_ID = String(IsExtensible.property("definition").id);
export const NODE_ARCHIVED_AT_KEY = String(IsArchivable.property("archived_at").id);
export const NODE_DELETED_AT_KEY = String(IsDeletable.property("deleted_at").id);

export const NODE_REFERENCE_TYPE_KEY = String(NodeReference.property("type").id);
export const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);
export const NODE_REFERENCE_SPACE_ID_KEY = String(NodeReference.property("space_id").id);
export const NODE_REFERENCE_DEFINITION_ID_KEY = String(NodeReference.property("definition_id").id);

export const ENTITY_SNAPSHOT_KEY = String(Entity.property("snapshot").id);
export const ENTITY_MATERIALIZATION_KEY = String(Entity.property("materialization").id);

export const EVENT_CREATED_AT_KEY = String(Event.property("created_at").id);
export const EVENT_SNAPSHOT_KEY = String(Event.property("snapshot").id);

/** Node.id + Node.snapshotId */
export interface VersionedNodeKey {
  id: string;
  snapshotId: string | null;
}

/** An in-memory database for Entities and Events. */
export class MemoryDatabase {
  public entityTables: Map<NodeType, MemoryEntityTable>;
  public eventTables: Map<NodeType, MemoryEventTable>;

  constructor() {
    this.entityTables = new Map();
    this.eventTables = new Map();

    // init tables
    for (const nodeClass of Object.values(NODE_CLASS_BY_TYPE)) {
      if (nodeClass.__definition__.isAbstract) {
        continue;
      } else if (nodeClass.__definition__.storeDomain == StoreDomain.ENTITY) {
        this.entityTables.set(nodeClass.metatype, new MemoryEntityTable(this, nodeClass.metatype));
      } else if (nodeClass.__definition__.storeDomain == StoreDomain.EVENT) {
        this.eventTables.set(nodeClass.metatype, new MemoryEventTable(this, nodeClass.metatype));
      } else {
        throw new Error(`unknown store domain for ${nodeClass.metatype}`);
      }
    }
  }

  toString(): string {
    const numEntities = Array.from(this.entityTables.values()).reduce(
      (sum, table) => sum + table.rows.size,
      0,
    );
    const numEvents = Array.from(this.eventTables.values()).reduce(
      (sum, table) => sum + table.rows.size,
      0,
    );
    return `entities=${numEntities}, events=${numEvents}, tables=${this.entityTables.size}`;
  }

  repr(): string {
    return `<MemoryDatabase ${this.toString()}>`;
  }
}

/** The current context for working with an in-memory database. */
export class MemoryContext {
  public database: MemoryDatabase;

  constructor(database: MemoryDatabase) {
    this.database = database;
  }

  toString(): string {
    return `entityTables=${this.database.entityTables.size}, eventTables=${this.database.eventTables.size}`;
  }

  repr(): string {
    return `<MemoryContext ${this.toString()}>`;
  }

  /** Expand the (separately) stored definitions for a NodeDefinition. */
  resolve(definition: NodeDefinitionReference): NodeDefinitionReference[] {
    return getSubdefinitionsForNodeType(definition.nodeType);
  }

  /** Get the entity table for a NodeDefinition. */
  getEntityTable(definition: NodeDefinitionReference | NodeReference): MemoryEntityTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    if (!this.database.entityTables.has(nodeType)) {
      throw new Error(`entity table for ${nodeType} not found in ${this.repr()}`);
    }
    return this.database.entityTables.get(nodeType)!;
  }

  /** Get the event table for a NodeDefinition. */
  getEventTable(definition: NodeDefinitionReference | NodeReference): MemoryEventTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    if (!this.database.eventTables.has(nodeType)) {
      throw new Error(`event table for ${nodeType} not found in ${this.repr()}`);
    }
    return this.database.eventTables.get(nodeType)!;
  }

  copy(): MemoryContext {
    return new MemoryContext(this.database);
  }
}
