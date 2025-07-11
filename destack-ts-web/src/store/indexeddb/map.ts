import {
  ENTITY_INDEXED_KEYS,
  EVENT_INDEXED_KEYS,
  IndexedDBEntityTable,
  IndexedDBEventTable,
  IndexedDBSchema,
} from "@destack-web/store/indexeddb/core";
import { NODE_CLASS_BY_TYPE, NodeType, StoreDomain } from "destack";

/** Get the IndexedDB store name for a node type. */
export function getNodeStoreName(nodeType: NodeType): string {
  return `destack_${nodeType}`;
}

/** Get the primary key for an Entity row. */
export function getEntityKey(nodeId: string, snapshotId: string | null): string {
  return `${nodeId}:${snapshotId || "<root>"}`;
}

/** Get the IndexedDB index name for a table and indexed key. */
export function getIndexName(
  table: IndexedDBEntityTable | IndexedDBEventTable,
  indexedKey: string,
): string {
  return `${table.name}_${indexedKey}`;
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
