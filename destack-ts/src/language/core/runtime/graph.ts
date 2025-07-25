import type { Entity, Event, NodeType } from "@destack/language/core/builtin";

/**
 * A Graph is a collection of Nodes from one or multiple Spaces (across time).
 */
export abstract class Graph {
  repr(): string {
    return `<${this.constructor.name}>`;
  }

  //
  // Meta
  //

  /** Open the Graph. */
  abstract open(): Promise<void>;

  /** Close the Graph. */
  abstract close(): Promise<void>;

  //
  // Write
  //

  /** Create a Snapshot. */
  abstract snapshot(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
    epoch: number;
  }): any;

  /** Insert Entities into the Graph directly. */
  abstract insert(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
    entities: Entity[];
  }): void;

  /** Append Events to the Graph. EditEvents are reflected immediately. */
  abstract append(events: Event[]): void;

  /** Restate Events to the Graph. EditEvents are reflected immediately. */
  abstract restate(events: Event[]): void;

  /** Prune the Graph. */
  abstract prune(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
  }): Promise<void>;

  /** Ensure Events/Entities are persisted in the Graph. */
  abstract commit(): Promise<void>;

  //
  // Read
  //

  /** Seek Events from the Graph. */
  abstract seek(options: {
    spaceId: string;
    branchId: string | null;
    snapshotId: string | null;
    type?: NodeType | NodeType[] | null;
    after?: Date | number | null;
    before?: Date | number | null;
  }): Event[];

  /** Get an Entity by id. */
  abstract get(options: {
    id: string;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    includeDeleted?: boolean;
  }): Entity | null;

  /** Get an Entity by id, raising an error if not found. */
  getOrError(options: {
    id: string;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    includeDeleted?: boolean;
  }): Entity {
    const node = this.get(options);
    if (node === null) {
      throw new Error(`node ${options.id} not found in ${this.repr()}`);
    }
    return node;
  }

  /** Collect child Entities (one level down). */
  abstract getChildren(options: {
    node: Entity;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    type?: NodeType | null;
    includeDeleted?: boolean;
  }): Entity[];

  /** Get the ancestors of this Entity (recursively up). */
  abstract getAncestors(options: {
    node: Entity;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    type?: NodeType | null;
    includeDeleted?: boolean;
  }): Entity[];

  /** Collect descendant Entities (recursively down). */
  abstract getDescendants(options: {
    node: Entity;
    spaceId: string;
    branchId: string;
    snapshotId: string;
    type?: NodeType | null;
    includeDeleted?: boolean;
  }): Entity[];
}
