import { Change, ChangeResult, Query, QueryResult, Store } from "@destack/language";

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
}
