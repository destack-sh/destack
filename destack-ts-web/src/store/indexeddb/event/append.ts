import { IndexedDBContext } from "@destack-web/store/indexeddb/core";
import { packEventRow } from "@destack-web/store/indexeddb/event/wiring";
import { Event, NODE_TYPE_SCALAR_BY_TYPE, Value } from "destack";
import { IDBPTransaction } from "idb";

/**
 * Append Events in IndexedDB.
 */
export async function executeAppend(options: {
  tx: IDBPTransaction<unknown, string[], "readwrite">;
  context: IndexedDBContext;
  events: Event[];
}): Promise<Event[]> {
  const { tx, context, events } = options;
  const promises: Promise<any>[] = [];
  for (const event of events) {
    const nodePtr = event.toRef();
    const table = context.getEventTable(nodePtr);
    const store = tx.objectStore(table.name);
    const eventValue = new Value({
      type: NODE_TYPE_SCALAR_BY_TYPE[nodePtr.type],
      value: event.toCson(),
    });
    const row = packEventRow(eventValue);
    promises.push(store.put(row));
  }
  await Promise.all(promises);
  return events;
}
