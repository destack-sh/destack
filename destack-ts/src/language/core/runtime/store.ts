import {
  Change,
  ChangeResult,
  Query,
  QueryResult,
  QueryUpdate,
} from "@destack/language/core/common";

/**
 * The read/write Store backing (part of) the Supergraph.
 * Could be a primary or secondary Storage from Databases or search or whatever.
 * Some Stores only support a subset of Edits.
 */
export abstract class Store {
  /**
   * Repr the Store.
   */
  abstract repr(): string;

  /**
   * Query the Store.
   */
  abstract query(query: Query): Promise<QueryResult>;

  /**
   * Commit the Changes as individual transactions (every Change is atomic by itself).
   */
  abstract commit(changes: Change[]): Promise<ChangeResult[]>;

  /**
   * Subscribe to a Query in the Store.
   */
  abstract subscribe(query: Query): AsyncIterator<QueryUpdate>;
}
