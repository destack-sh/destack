import type {
  EditEvent,
  Event,
  NodeType,
  Query,
  QueryResult,
  QueryUpdate,
} from "@destack/language";
import { StoreType } from "@destack/language/core/builtin/common";
import { NODE_TYPES_BY_PRIMARY_STORE_TYPE } from "@destack/language/registry";

/**
 * The read/write Store backing (part of) the Supergraph.
    Some Stores only support a subset of Entities/Events.
 */
export abstract class Store {
  /**
   * The StoreTypes this Store represents.
   */
  readonly types: StoreType[];

  /**
   * The NodeTypes this Store supports.
   */
  readonly nodeTypes: NodeType[];

  constructor(options: { types: StoreType[] }) {
    this.types = options.types;

    const nodeTypes: NodeType[] = [];
    for (const type of options.types) {
      for (const nodeType of NODE_TYPES_BY_PRIMARY_STORE_TYPE[type]) {
        if (!nodeTypes.includes(nodeType)) {
          nodeTypes.push(nodeType);
        }
      }
    }
    this.nodeTypes = nodeTypes;
  }

  /**
   * Repr the Store.
   */
  abstract repr(): string;

  /**
   * Query the Store.
   */
  abstract query(query: Query): Promise<QueryResult>;
}

/**
 * A Store for Entities.
 */
export abstract class EntityStore extends Store {
  /**
   * Commit the EditEvents.
   */
  abstract commit(events: EditEvent[]): Promise<EditEvent[]>;
}

/**
 * A Store for Events (technically a supserset of EntityStore).
 */
export abstract class EventStore extends Store {
  /**
   * Commit the Events.
   */
  abstract append(events: Event[]): Promise<Event[]>;
}

/**
 * A Store that supports Query subscriptions.
 */
export abstract class LiveStore extends EventStore {
  /**
   * Subscribe to a Query in the Store.
   */
  abstract subscribe(query: Query): AsyncIterator<QueryUpdate>;
}
