import { Change, ChangeResult, Query, QueryResult, QueryUpdate, Store } from "@destack/language";

/** An in-memory Store. */
export class MemoryStore implements Store {
  repr(): string {
    throw new Error("Method not implemented.");
  }

  query(query: Query): Promise<QueryResult> {
    throw new Error("Method not implemented.");
  }
  
  commit(changes: Change[]): Promise<ChangeResult[]> {
    throw new Error("Method not implemented.");
  }

  subscribe(query: Query): AsyncIterator<QueryUpdate, any, any> {
    throw new Error("Method not implemented.");
  }
}
