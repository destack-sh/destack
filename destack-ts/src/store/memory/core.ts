import {
  Entity,
  Event,
  IsExtensible,
  IsSpatial,
  Node,
  NodeDefinitionReference,
  NodeDefinitionType,
  NodeReference,
  NodeType,
} from "@destack/language";
import { NODE_CLASS_BY_TYPE } from "@destack/language/registry";
import { assertNever } from "@destack/utils";
import { MemoryEntityTable } from "./entity/core";
import { MemoryEventTable } from "./event/core";

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

export const EVENT_CREATED_AT_KEY = String(Event.property("created_at").id);
export const EVENT_SNAPSHOT_PTR_KEY = String(Event.property("snapshot").id);

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

/** A context for evaluating queries against an in-memory database. */
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
    if (definition.type === NodeDefinitionType.BUILTIN) {
      if (!definition.nodeType) {
        throw new Error(`no node_type for ${definition.repr()}`);
      }
      const nodeClass = NODE_CLASS_BY_TYPE[definition.nodeType];
      const nodeDefinition = nodeClass.__definition__;
      if (nodeDefinition.inheritedBy.length === 0) {
        return [definition];
      }
      const subdefinitions: NodeDefinitionReference[] = [];
      if (!nodeDefinition.isAbstract) {
        subdefinitions.push(definition);
      }
      for (const subnodeType of nodeDefinition.inheritedBy) {
        const subnodeClass = NODE_CLASS_BY_TYPE[subnodeType];
        const subnodeDefinition = subnodeClass.__definition__;
        if (!subnodeDefinition.isAbstract) {
          subdefinitions.push(NodeDefinitionReference.of(subnodeClass));
        }
      }
      return subdefinitions;
    } else if (definition.type === NodeDefinitionType.CUSTOM) {
      throw new Error(`cannot resolve ${definition.repr()}`);
    } else {
      assertNever(definition.type);
    }
  }

  /** Get the entity table for a NodeDefinition. */
  getEntityTable(definition: NodeDefinitionReference | NodeReference): MemoryEntityTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    if (!this.database.entityTables.has(nodeType)) {
      this.database.entityTables.set(nodeType, new MemoryEntityTable(this.database, nodeType));
    }
    return this.database.entityTables.get(nodeType)!;
  }

  /** Get the event table for a NodeDefinition. */
  getEventTable(definition: NodeDefinitionReference | NodeReference): MemoryEventTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    if (!this.database.eventTables.has(nodeType)) {
      this.database.eventTables.set(nodeType, new MemoryEventTable(this.database, nodeType));
    }
    return this.database.eventTables.get(nodeType)!;
  }

  copy(): MemoryContext {
    return new MemoryContext(this.database);
  }
}
