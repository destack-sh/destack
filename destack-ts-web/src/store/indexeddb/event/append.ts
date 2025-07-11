import { IndexedDBContext } from "@destack-web/store/indexeddb/core";
import { packEventRow } from "@destack-web/store/indexeddb/event/wiring";
import { Event, NODE_TYPE_SCALAR_BY_TYPE, Value } from "destack";
import { IDBPTransaction } from "idb";

/**
 * Append Events in IndexedDB.
 */
export function executeAppend(
  tx: IDBPTransaction<unknown, string[], "readwrite">,
  context: IndexedDBContext,
  events: Event[],
): Event[] {
  for (const event of events) {
    const eventPtr = event.toRef();
    const eventTable = context.getEventTable(eventPtr);
    const eventType = NODE_TYPE_SCALAR_BY_TYPE[eventPtr.type];
    const eventValue = new Value({ type: eventType, value: event.toValue() });
    const eventRow = packEventRow(eventValue);
    tx.objectStore(eventTable.name).put(eventRow);
  }
  return events;
}
