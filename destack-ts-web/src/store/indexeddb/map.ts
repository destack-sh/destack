import {
  ENTITY_INDEXED_KEYS,
  EVENT_INDEXED_KEYS,
  IndexedDBEntityTable,
  IndexedDBEventTable,
  IndexedDBSchema,
} from "@destack-web/store/indexeddb/core";
import { NODE_CLASS_BY_TYPE, NodeType, StoreDomain } from "destack";

function getNodeStoreName(nodeType: NodeType): string {
  return `destack_${nodeType}`;
}

export function getIndexedDBSchema(): IndexedDBSchema {
  // entities: every (concrete) entity gets its own table
  const entityTables = new Map<NodeType, IndexedDBEntityTable>();
  for (const nodeType of Object.values(NODE_CLASS_BY_TYPE)) {
    if (nodeType.__definition__.isAbstract) {
      continue;
    } else if (nodeType.__definition__.storeDomain == StoreDomain.ENTITY) {
      const entityTable = new IndexedDBEntityTable({
        nodeType: nodeType.metatype,
        name: getNodeStoreName(nodeType.metatype),
        indexedKeys: ENTITY_INDEXED_KEYS,
      });
      entityTables.set(nodeType.metatype, entityTable);
    }
  }

  // events: all events go in one table
  const eventTable = new IndexedDBEventTable({
    nodeType: NodeType.EVENT,
    name: getNodeStoreName(NodeType.EVENT),
    indexedKeys: EVENT_INDEXED_KEYS,
  });
  const eventTables = new Map<NodeType, IndexedDBEventTable>();
  eventTables.set(NodeType.EVENT, eventTable);

  const schema = new IndexedDBSchema({ entityTables, eventTables });
  return schema;
}
