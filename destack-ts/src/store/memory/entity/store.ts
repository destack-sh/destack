import {
  EditEvent,
  EntityStore,
  getNodeTypesForStores,
  NodeType,
  Query,
  QueryResult,
  StoreKey,
} from "@destack/language";
import { MemoryContext, MemoryDatabase } from "@destack/store/memory/core";
import { executeEdits } from "@destack/store/memory/entity/edit";
import { executeQuery } from "@destack/store/memory/entity/query";

/** An in-memory Store for Entities. */
export class MemoryEntityStore implements EntityStore {
  public types: StoreKey[];
  public nodeTypes: NodeType[];
  public database: MemoryDatabase;
  public context: MemoryContext;

  constructor(options: { types: StoreKey[]; database?: MemoryDatabase }) {
    this.types = options.types;
    this.nodeTypes = getNodeTypesForStores(this.types);
    this.database = options.database ?? new MemoryDatabase();
    this.context = new MemoryContext(this.database);
  }

  toString(): string {
    let numNodes = 0;
    for (const table of this.database.entityTables.values()) {
      numNodes += table.rows.size;
    }
    return `nodes=${numNodes}`;
  }

  repr(): string {
    return `<MemoryEntityStore ${this.toString()}>`;
  }

  async query(query: Query): Promise<QueryResult> {
    const result = executeQuery({ context: this.context, query });
    return result;
  }

  async commit(events: EditEvent[]): Promise<EditEvent[]> {
    const { edits, cascadedEdits } = executeEdits({
      context: this.context,
      edits: events,
    });
    const appliedEdits = [...edits, ...cascadedEdits];
    return appliedEdits;
  }
}
