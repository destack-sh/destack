import { NodeReference, NodeType } from "@destack/language";
import { MemoryDatabase, VersionedNodeKey } from "@destack/store/memory/core";

export class MemoryEventTable {
  public database: MemoryDatabase;
  public nodeType: NodeType;
  public rows: Map<string, MemoryEventRow>; // key is id
  public rowsSorted: MemoryEventRow[]; // sorted by created_at

  constructor(database: MemoryDatabase, nodeType: NodeType) {
    this.database = database;
    this.nodeType = nodeType;
    this.rows = new Map();
    this.rowsSorted = [];
  }

  toString(): string {
    return `nodeType=${NodeType[this.nodeType]}, rows=${this.rows.size}`;
  }

  repr(): string {
    return `<MemoryEventTable ${this.toString()}>`;
  }

  /** Convert a VersionedNodeKey to a string key. */
  getNodeKey(nodeKey: VersionedNodeKey): string {
    return `${nodeKey.id}:${nodeKey.snapshotId || "<root>"}`;
  }
}

export class MemoryEventRow {
  public metatype: NodeType;
  public id: string;
  public snapshotId: string | null;
  public ptr: NodeReference;
  public createdAt: Date;
  public value: Record<string, any>;

  constructor(options: {
    metatype: NodeType;
    id: string;
    snapshotId: string | null;
    ptr: NodeReference;
    createdAt: Date;
    value: Record<string, any>;
  }) {
    this.metatype = options.metatype;
    this.id = options.id;
    this.snapshotId = options.snapshotId;
    this.ptr = options.ptr;
    this.createdAt = options.createdAt;
    this.value = options.value;
  }

  toString(): string {
    return `nodeType${this.metatype}, id=${this.id}, value=${Object.keys(this.value).length}`;
  }

  repr(): string {
    return `<MemoryEventRow ${this.toString()}>`;
  }
}
