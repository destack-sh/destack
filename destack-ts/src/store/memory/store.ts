import {
  Change,
  ChangeResult,
  ChangeStatus,
  Query,
  QueryResult,
  QueryUpdate,
  Store,
  StoreImplementation,
  StoreType,
} from "@destack/language";

import { MemoryContext, MemoryDatabase } from "./core";
import { executeChange } from "./edit";
import { executeQuery } from "./query";

/** An in-memory Store. */
export class MemoryStore extends Store {
  public static readonly implementation: StoreImplementation = StoreImplementation.MEMORY;

  public database: MemoryDatabase;
  public context: MemoryContext;

  constructor(options: { types: StoreType[] }) {
    super(options);
    this.database = new MemoryDatabase();
    this.context = new MemoryContext(this.database);
  }

  toString(): string {
    const numNodes = Array.from(this.database.tables.values()).reduce(
      (sum, table) => sum + table.rows.size,
      0,
    );
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

  async commit(changes: Change[]): Promise<ChangeResult[]> {
    const results: ChangeResult[] = [];
    for (const change of changes) {
      const { edits, cascadedEdits } = executeChange({
        database: this.database,
        context: this.context,
        change,
      });
      const result = new ChangeResult({
        id: change.id,
        status: ChangeStatus.COMPLETED,
        edits: edits,
        cascadedEdits: cascadedEdits,
      });
      results.push(result);
    }
    return results;
  }

  subscribe(query: Query): AsyncIterator<QueryUpdate, any, any> {
    throw new Error(`${this.repr()} does not support subscribe: ${query.repr()}`);
  }
}
