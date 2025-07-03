import {
  Change,
  ChangeResult,
  Query,
  QueryResult,
  QueryUpdate,
  Store,
  StoreType,
} from "@destack/language";

/** An in-memory Store. */
export class MemoryStore extends Store {
  constructor(options: { types: StoreType[] }) {
    super(options);
  }

  repr(): string {
    return `<MemoryStore>`;
  }

  query(query: Query): Promise<QueryResult> {
    throw new Error(`${this.repr()} does not support query: ${query.repr()}`);
  }

  commit(changes: Change[]): Promise<ChangeResult[]> {
    throw new Error(
      `${this.repr()} does not support commit: ${changes.map((c) => c.repr()).join(", ")}`,
    );
  }

  subscribe(query: Query): AsyncIterator<QueryUpdate, any, any> {
    throw new Error(`${this.repr()} does not support subscribe: ${query.repr()}`);
  }
}
