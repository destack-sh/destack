import {
  Materialization,
  NodeDefinitionReference,
  NodeDefinitionType,
  NodeReference,
  NodeType,
} from "@destack/language";
import { NODE_CLASS_BY_TYPE } from "@destack/language/registry";
import { assertNever } from "@destack/utils";

export interface VersionedNodeKey {
  id: string;
  snapshotId: string | null;
}

export class MemoryDatabase {
  public tables: Map<NodeType, MemoryTable>;

  constructor() {
    this.tables = new Map();
  }

  toString(): string {
    const numNodes = Array.from(this.tables.values()).reduce(
      (sum, table) => sum + table.rows.size,
      0,
    );
    return `nodes=${numNodes}, tables=${this.tables.size}`;
  }

  repr(): string {
    return `<MemoryDatabase ${this.toString()}>`;
  }
}

export class MemoryContext {
  public database: MemoryDatabase;

  constructor(database: MemoryDatabase) {
    this.database = database;
  }

  toString(): string {
    return `tables=${this.database.tables.size}`;
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

  /** Get the Table for a NodeDefinition. */
  get(definition: NodeDefinitionReference | NodeReference): MemoryTable {
    const nodeType = definition instanceof NodeReference ? definition.type : definition.nodeType;
    if (!this.database.tables.has(nodeType)) {
      this.database.tables.set(nodeType, new MemoryTable(this.database, nodeType));
    }
    return this.database.tables.get(nodeType)!;
  }

  copy(): MemoryContext {
    return new MemoryContext(this.database);
  }
}

export class MemoryTable {
  public database: MemoryDatabase;
  public nodeType: NodeType;
  public rows: Map<string, MemoryRow>; // key is serialized VersionedNodeKey
  public rowsByParent: Map<string, MemoryRow[]>; // key is serialized VersionedNodeKey
  public rowsBySnapshot: Map<string | null, Map<string, MemoryRow>>; // first key is snapshot_id, second is node_id

  constructor(database: MemoryDatabase, nodeType: NodeType) {
    this.database = database;
    this.nodeType = nodeType;
    this.rows = new Map();
    this.rowsByParent = new Map();
    this.rowsBySnapshot = new Map();
  }

  toString(): string {
    return `nodeType=${NodeType[this.nodeType]}, rows=${this.rows.size}`;
  }

  repr(): string {
    return `<MemoryTable ${this.toString()}>`;
  }

  /** Convert a VersionedNodeKey to a string key. */
  getNodeKey(nodeKey: VersionedNodeKey): string {
    return `${nodeKey.id}:${nodeKey.snapshotId || "<root>"}`;
  }
}

export class MemoryRow {
  public table: MemoryTable;
  public nodeType: NodeType;
  public id: string;
  public snapshotId: string | null;
  public materialization: Materialization | null;
  public ptr: NodeReference;
  public parentPtr: NodeReference | null;
  public value: Record<string, any>;

  constructor(
    table: MemoryTable,
    nodeType: NodeType,
    id: string,
    snapshotId: string | null,
    materialization: Materialization | null,
    ptr: NodeReference,
    parentPtr: NodeReference | null,
    value: Record<string, any>,
  ) {
    this.table = table;
    this.nodeType = nodeType;
    this.id = id;
    this.snapshotId = snapshotId;
    this.materialization = materialization;
    this.ptr = ptr;
    this.parentPtr = parentPtr;
    this.value = value;
  }

  toString(): string {
    return `nodeType${this.nodeType}, id=${this.id}, value=${Object.keys(this.value).length}`;
  }

  repr(): string {
    return `<MemoryRow ${this.toString()}>`;
  }
}
