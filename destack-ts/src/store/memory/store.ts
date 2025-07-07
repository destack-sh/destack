import {
  EditEvent,
  EntityStore,
  Query,
  QueryResult,
  QueryUpdate,
  StoreImplementation,
  StoreType,
} from "@destack/language";
import { MemoryContext, MemoryDatabase } from "@destack/store/memory/core";
import { executeChange } from "@destack/store/memory/edit";
import { executeQuery } from "@destack/store/memory/query";

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

  async query(query: Query): Promise<QueryResult> {
    const result = executeQuery({
      context: this.context,
      query,
    });
    return result;
  }

  async commit(events: EditEvent[]): Promise<EditEvent[]> {
    const { edits, cascadedEdits } = executeChange({
      database: this.database,
      context: this.context,
      edits: events,
    });
    const appliedEdits = [...edits, ...cascadedEdits];
    return appliedEdits;
  }

  subscribe(query: Query): AsyncIterator<QueryUpdate, any, any> {
    throw new Error(`${this.repr()} does not support subscribe: ${query.repr()}`);
  }
}
