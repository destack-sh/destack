import {
  EditEvent,
  EntityStore,
  Event,
  EventStore,
  Query,
  QueryResult,
  StoreImplementation,
  StoreType,
  toValue,
} from "@destack/language";
import { MemoryContext, MemoryDatabase, MemoryTable } from "@destack/store/memory/core";
import { executeEdits } from "@destack/store/memory/edit";
import { executeQuery } from "@destack/store/memory/query";
import { packNodeRow } from "@destack/store/memory/wiring";

/** An in-memory Store. */
export class MemoryEntityStore extends EntityStore {
  public static readonly implementation: StoreImplementation = StoreImplementation.MEMORY;

  public database: MemoryDatabase;
  public context: MemoryContext;

  constructor(options: { types: StoreType[] }) {
    super(options);
    this.database = new MemoryDatabase();
    this.context = new MemoryContext(this.database);
  }

  toString(): string {
    let numNodes = 0;
    for (const table of this.database.tables.values()) {
      numNodes += table.rows.size;
    }
    return `nodes=${numNodes}, tables=${this.database.tables.size}`;
  }

  repr(): string {
    return `<MemoryStore ${this.toString()}>`;
  }

  override async query(query: Query): Promise<QueryResult> {
    const result = executeQuery({
      context: this.context,
      query,
    });
    return result;
  }

  override async commit(events: EditEvent[]): Promise<EditEvent[]> {
    const { edits, cascadedEdits } = executeEdits({
      context: this.context,
      edits: events,
    });
    const appliedEdits = [...edits, ...cascadedEdits];
    return appliedEdits;
  }
}

/** An in-memory Store for Events. */
export class MemoryEventStore extends EventStore {
  public static readonly implementation: StoreImplementation = StoreImplementation.MEMORY;

  public database: MemoryDatabase;
  public context: MemoryContext;

  constructor(options: { types: StoreType[] }) {
    super(options);
    this.database = new MemoryDatabase();
    this.context = new MemoryContext(this.database);
  }

  toString(): string {
    let numNodes = 0;
    for (const table of this.database.tables.values()) {
      numNodes += table.rows.size;
    }
    return `nodes=${numNodes}, tables=${this.database.tables.size}`;
  }

  repr(): string {
    return `<MemoryEventStore ${this.toString()}>`;
  }

  override async query(query: Query): Promise<QueryResult> {
    const result = executeQuery({
      context: this.context,
      query,
    });
    return result;
  }

  override async append(events: Event[]): Promise<Event[]> {
    for (const event of events) {
      const nodeType = event.metatype;
      if (!this.database.tables.has(nodeType)) {
        this.database.tables.set(nodeType, new MemoryTable(this.database, nodeType));
      }
      const table = this.database.tables.get(nodeType)!;
      const eventValue = toValue(event, undefined, { nodeAsValue: true });
      const row = packNodeRow(table, eventValue);
      const rowKey = table.getNodeKey(row);
      table.rows.set(rowKey, row);
    }
    return events;
  }
}
