import { getPostgres } from "@desys/store/postgres/entity/client";
import { PostgresContext } from "@desys/store/postgres/entity/core";
import { executeEdits } from "@desys/store/postgres/entity/edit";
import { executeQuery } from "@desys/store/postgres/entity/query";
import { SQL } from "bun";
import {
  EditEvent,
  EntityStore,
  getNodeTypesForStores,
  NodeType,
  Query,
  QueryResult,
  StoreKey,
} from "destack";

/** An EntityStore implementation that uses Postgres as the backend. */
export class PostgresEntityStore implements EntityStore {
  keys: StoreKey[];
  nodeTypes: NodeType[];

  dbInfo: {
    url: string;
    tls?: boolean;
  };
  db: SQL | null;
  context: PostgresContext;

  constructor(options: {
    database: {
      url: string;
      tls?: boolean;
    };
    keys: StoreKey[];
  }) {
    this.keys = options.keys;
    this.nodeTypes = getNodeTypesForStores(options.keys);

    this.dbInfo = options.database;
    this.db = null;
    this.context = new PostgresContext(options.keys);
  }

  toString(): string {
    return `database=${this.db?.name}`;
  }

  repr(): string {
    return `<PostgresEntityStore ${this.toString()}>`;
  }

  async open(): Promise<void> {
    this.db = await getPostgres(this.dbInfo);

    // nocheckin: migrations :Migration
    await this.db.begin(async (tx) => {
      for (const table of this.context.tablesByName.values()) {
        const sql = table.sql();
        await tx.unsafe(sql);
      }
    });
  }

  async close(): Promise<void> {
    if (this.db) {
      await this.db.close();
      this.db = null;
    }
  }

  async query(query: Query): Promise<QueryResult> {
    if (!this.db) {
      throw new Error(`${this.repr()} is not open`);
    }
    const { result } = await executeQuery({
      tx: this.db,
      context: this.context,
      query,
    });
    return result;
  }

  async commit(events: EditEvent[]): Promise<EditEvent[]> {
    if (!this.db) {
      throw new Error(`${this.repr()} is not open`);
    }
    const appliedEdits = await this.db.begin(async (tx) => {
      const { edits, cascadedEdits } = await executeEdits({
        tx,
        context: this.context,
        edits: events,
      });
      const appliedEdits = [...edits, ...cascadedEdits];
      return appliedEdits;
    });
    return appliedEdits;
  }
}
