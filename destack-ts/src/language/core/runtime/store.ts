import type {
  EditEvent,
  Event,
  NodeType,
  Query,
  QueryResult,
  QueryUpdate,
} from "@destack/language";
import { StoreType } from "@destack/language/core/builtin/common";

/**
 * The read/write Store backing (part of) the Supergraph.
    Some Stores only support a subset of Entities/Events.
 */
export interface Store {
  /**
   * The StoreTypes this Store represents.
   */
  readonly types: StoreType[];

  /**
   * The NodeTypes this Store supports.
   */
  readonly nodeTypes: NodeType[];

  /**
   * Repr the Store.
   */
  repr(): string;

  /**
   * Query the Store.
   */
  query(query: Query): Promise<QueryResult>;
}

/**
 * A Store for Entities.
 */
export interface EntityStore extends Store {
  /**
   * Commit the EditEvents.
   */
  commit(events: EditEvent[]): Promise<EditEvent[]>;
}

/**
 * A Store for Events (technically a supserset of EntityStore).
 */
export interface EventStore extends Store {
  /**
   * Commit the Events.
   */
  append(events: Event[]): Promise<Event[]>;
}

/**
 * A Store that supports Query subscriptions.
 */
export interface LiveStore extends Store {
  /**
   * Subscribe to a Query in the Store.
   */
  subscribe(query: Query): AsyncIterator<QueryUpdate>;
}
