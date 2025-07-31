import type { Entity, Event, NodeType } from "@destack/language/core/builtin";
import { Graph } from "@destack/language/core/runtime/graph";

// nocheckin(ts): implement FileGraph

/** A Graph that stores Nodes in files. */
export class FileGraph extends Graph {
  private nodesById: Map<string, Entity>;

  constructor() {
    super();
    this.nodesById = new Map();
  }

  //
  // Meta
  //

  override async open(): Promise<void> {
    // nothing to do
  }

  override async close(): Promise<void> {
    // nothing to do
  }

  //
  // Write
  //

  override snapshot(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
    epoch: number;
  }): any {
    throw new Error("not implemented");
  }

  override insert(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
    entities: Entity[];
  }): void {
    throw new Error("not implemented");
  }

  override append(events: Event[]): void {
    throw new Error("not implemented");
  }

  override restate(events: Event[]): void {
    throw new Error("not implemented");
  }

  override async prune(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
  }): Promise<void> {
    throw new Error("not implemented");
  }

  override async commit(): Promise<void> {
    throw new Error("not implemented");
  }

  //
  // Read
  //

  override seek(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
    type?: NodeType | NodeType[] | null;
    after?: Date | number | null;
    before?: Date | number | null;
  }): Event[] {
    throw new Error("not implemented");
  }

  override get(options: {
    id: string;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    includeDeleted?: boolean;
  }): Entity | null {
    return this.nodesById.get(options.id) ?? null;
  }

  override getChildren(options: {
    node: Entity;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    type?: NodeType | null;
    includeDeleted?: boolean;
  }): Entity[] {
    throw new Error("not implemented");
  }

  override getAncestors(options: {
    node: Entity;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    type?: NodeType | null;
    includeDeleted?: boolean;
  }): Entity[] {
    throw new Error("not implemented");
  }

  override getDescendants(options: {
    node: Entity;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    type?: NodeType | null;
    includeDeleted?: boolean;
  }): Entity[] {
    throw new Error("not implemented");
  }
}
