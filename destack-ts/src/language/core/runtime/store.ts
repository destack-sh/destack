import type {
  Change,
  ChangeResult,
  NodeType,
  Query,
  QueryResult,
  QueryUpdate,
} from "@destack/language";
import { StoreType } from "@destack/language/core/builtin/common";
import { NODE_TYPES_BY_PRIMARY_STORE_TYPE } from "@destack/language/registry";

/**
 * The read/write Store backing (part of) the Supergraph.
 * Some Stores only support a subset of Edits.
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

  /**
   * Commit the Changes as individual transactions (every Change is atomic by itself).
   */
  abstract commit(changes: Change[]): Promise<ChangeResult[]>;

  /**
   * Subscribe to a Query in the Store.
   */
  abstract subscribe(query: Query): AsyncIterator<QueryUpdate>;
}
