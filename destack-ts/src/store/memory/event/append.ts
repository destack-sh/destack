import { Event, NodeType, toValue } from "@destack/language";
import { NODE_CLASS_BY_TYPE } from "@destack/language/registry";
import { MemoryContext } from "../core";
import { packEventRow } from "./wiring";

/**
 * Append events to the hierarchical event tables.
 */
export function executeAppend(context: MemoryContext, events: Event[]): Event[] {
  for (const event of events) {
    const nodeType = event.metatype as NodeType;
    const eventValue = toValue(event, undefined, { nodeAsValue: true });
    if (!eventValue.value) {
      throw new Error(`no value for ${event.repr()}`);
    }
    const eventRow = packEventRow(eventValue);

    // append to all parent types in hierarchy (including self and event)
    let currentType = nodeType;
    while (currentType >= NodeType.EVENT) {
      const table = context.getEventTable(eventRow.ptr);

      // add to table
      table.rows.set(eventRow.id, eventRow);

      // insert into sorted list
      // binary search for insertion point
      let left = 0;
      let right = table.rowsSorted.length;
      while (left < right) {
        const mid = Math.floor((left + right) / 2);
        if (table.rowsSorted[mid].createdAt <= eventRow.createdAt) {
          left = mid + 1;
        } else {
          right = mid;
        }
      }
      table.rowsSorted.splice(left, 0, eventRow);

      // move up the hierarchy
      const nodeClass = NODE_CLASS_BY_TYPE[currentType];
      if (nodeClass.__definition__.baseType !== null) {
        currentType = nodeClass.__definition__.baseType;
      } else {
        break;
      }
    }
  }

  return events;
}
