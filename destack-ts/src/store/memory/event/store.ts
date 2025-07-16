import {
  Event,
  EventStore,
  getNodeTypesForStores,
  NodeType,
  Query,
  QueryResult,
  StoreKey,
} from "@destack/language";
import { MemoryContext, MemoryDatabase } from "@destack/store/memory/core";
import { executeAppend } from "@destack/store/memory/event/append";
import { executeQuery } from "@destack/store/memory/event/query";

/** An in-memory Store for Events. */
export class MemoryEventStore implements EventStore {
  public keys: StoreKey[];
  public nodeTypes: NodeType[];
  public database: MemoryDatabase;
  public context: MemoryContext;

  constructor(options: { keys: StoreKey[]; database?: MemoryDatabase }) {
    this.keys = options.keys;
    this.nodeTypes = getNodeTypesForStores(this.keys);
    this.database = options.database ?? new MemoryDatabase();
    this.context = new MemoryContext(this.database);
  }

  toString(): string {
    let numNodes = 0;
    for (const table of this.database.eventTables.values()) {
      numNodes += table.rows.size;
    }
    return `nodes=${numNodes}`;
  }

  repr(): string {
    return `<MemoryEventStore ${this.toString()}>`;
  }

  async open(): Promise<void> {
    // nothing to do
  }

  async close(): Promise<void> {
    // nothing to do
  }

  async query(query: Query): Promise<QueryResult> {
    const result = executeQuery(this.context, query);
    return result;
  }

  async append(events: Event[]): Promise<Event[]> {
    return executeAppend(this.context, events);
  }
}
