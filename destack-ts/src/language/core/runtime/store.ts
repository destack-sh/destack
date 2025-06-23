import { Change, ChangeResult, Query, QueryResult, QueryUpdate } from "@destack/language";

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
   * Returns a QueryResult.
   */
  abstract query(query: Query): Promise<QueryResult>;

  /**
   * Commit the Changes as individual transactions (every Change is atomic by itself).
   * Returns the ChangeResults per Change.
   */
  abstract commit(changes: Change[]): Promise<ChangeResult[]>;
}

/**
 * A Store that can be subscribed to for Query updates.
 */
export abstract class LiveStore extends Store {
  /** Subscribe to a Query in the Store. */
  abstract subscribe(query: Query): AsyncIterator<QueryUpdate>;
}

/**
 * A Store that can stage Changes optimistically.
 * To commit Changes (incl. staged), pass these Changes to Store.commit as usual.
 * To remove Changes without committing, call Store.unstage.
 */
export abstract class OptimisticStore extends LiveStore {
  /** Stage Changes locally. */
  abstract stage(changes: Change[]): Promise<void>;

  /** Unstage Changes locally. */
  abstract unstage(changes: Change[]): Promise<void>;
}
