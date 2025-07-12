import { IndexedDBStoreBase } from "@destack-web/store/indexeddb/core";
import { executeAppend } from "@destack-web/store/indexeddb/event/append";
import { executeQuery } from "@destack-web/store/indexeddb/event/query";
import { Event, EventStore, Query, QueryResult } from "@destack/language";
import { IDBPTransaction } from "idb";

/** An IndexedDB Store for Events. */
export class IndexedDBEventStore extends IndexedDBStoreBase implements EventStore {
  private eventCount: number | null = null;

  toString(): string {
    return `events=${this.eventCount ?? "<unknown>"}`;
  }

  repr(): string {
    return `<IndexedDBEventStore ${this.toString()}>`;
  }

  override async open(): Promise<void> {
    await super.open();
    if (this.db == null) {
      throw new Error(`database is not open in ${this.repr()}`);
    }

    // fetch initial event count
    let eventCount = 0;
    for (const eventTable of this.schema.eventTables.values()) {
      const count = await this.db.count(eventTable.name);
      eventCount += count;
    }
    this.eventCount = eventCount;
  }

  async query(
    query: Query,
    options?: {
      tx?: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
    },
  ): Promise<QueryResult> {
    if (this.db == null) {
      throw new Error(`store not open in ${this.repr()}`);
    }

    const eventTables = Array.from(this.schema.eventTables.values());
    const tx =
      options?.tx ??
      this.db.transaction(
        eventTables.map((table) => table.name),
        "readonly",
      );

    const result = await executeQuery({
      tx,
      context: this.context,
      query,
    });

    await tx.done;
    return result;
  }

  async append(
    events: Event[],
    options?: {
      tx?: IDBPTransaction<unknown, string[], "readwrite">;
    },
  ): Promise<Event[]> {
    if (this.eventCount == null || this.db == null) {
      throw new Error(`store not open in ${this.repr()}`);
    }

    // get transaction
    let tx: IDBPTransaction<unknown, string[], "readwrite">;
    if (options?.tx) {
      tx = options.tx;
    } else {
      const eventTables = Array.from(this.schema.eventTables.values());
      tx = this.db.transaction(
        eventTables.map((table) => table.name),
        "readwrite",
      );
    }
    await executeAppend({ tx, context: this.context, events });

    // commit the transaction if we created it
    if (options?.tx == null) {
      await tx.done;
    }

    // update event count
    this.eventCount += events.length;

    return events;
  }
}
