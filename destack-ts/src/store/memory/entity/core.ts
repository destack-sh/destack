import {
  Materialization,
  NodeReference,
  NodeType,
} from "@destack/language";
import { MemoryDatabase, VersionedNodeKey } from "../core";

export class MemoryEntityTable {
  public database: MemoryDatabase;
  public nodeType: NodeType;
  public rows: Map<string, MemoryEntityRow>; // key is serialized VersionedNodeKey
  public rowsByParent: Map<string, MemoryEntityRow[]>; // key is serialized VersionedNodeKey
  public rowsBySnapshot: Map<string | null, Map<string, MemoryEntityRow>>; // first key is snapshot_id, second is node_id

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
    return `<MemoryEntityTable ${this.toString()}>`;
  }

  /** Convert a VersionedNodeKey to a string key. */
  getNodeKey(nodeKey: VersionedNodeKey): string {
    return `${nodeKey.id}:${nodeKey.snapshotId || "<root>"}`;
  }
}

export class MemoryEntityRow {
  public metatype: NodeType;
  public id: string;
  public snapshotId: string | null;
  public materialization: Materialization | null;
  public ptr: NodeReference;
  public parentPtr: NodeReference | null;
  public value: Record<string, any>;

  constructor(options: {
    metatype: NodeType,
    id: string,
    snapshotId: string | null,
    materialization: Materialization | null,
    ptr: NodeReference,
    parentPtr: NodeReference | null,
    value: Record<string, any>,
  }) {
    this.metatype = options.metatype;
    this.id = options.id;
    this.snapshotId = options.snapshotId;
    this.materialization = options.materialization;
    this.ptr = options.ptr;
    this.parentPtr = options.parentPtr;
    this.value = options.value;
  }

  toString(): string {
    return `nodeType${this.metatype}, id=${this.id}, value=${Object.keys(this.value).length}`;
  }

  repr(): string {
    return `<MemoryEntityRow ${this.toString()}>`;
  }
} 