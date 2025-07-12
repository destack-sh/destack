import { IndexedDBContext } from "@destack-web/store/indexeddb/core";
import { packEventRow } from "@destack-web/store/indexeddb/event/wiring";
import { Event, NODE_TYPE_SCALAR_BY_TYPE, Value } from "destack";
import { IDBPTransaction } from "idb";

/**
 * Append Events in IndexedDB.
 */
export function executeAppend(options: {
  tx: IDBPTransaction<unknown, string[], "readwrite">;
  context: IndexedDBContext;
  events: Event[];
}): Event[] {
  const { tx, context, events } = options;
  for (const event of events) {
    const nodePtr = event.toRef();
    const table = context.getEventTable(nodePtr);
    const store = tx.objectStore(table.name);
    const eventValue = new Value({
      type: NODE_TYPE_SCALAR_BY_TYPE[nodePtr.type],
      value: event.toValue(),
    });
    const row = packEventRow(eventValue);
    store.put(row);
  }
  return events;
}
