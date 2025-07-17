import {
  EXTENSIONS,
  POSTGRES_BUILTIN_TABLE_PREFIX,
  PostgresColumn,
  PostgresConstraint,
  PostgresIndex,
  PostgresIndexType,
  PostgresSchema,
  PostgresTable,
} from "@desys/store/postgres/entity/core";
import {
  NODE_CLASS_BY_TYPE,
  NodeReference,
  NodeType,
  ScalarType,
  StoreKey,
  TypeCardinality,
} from "destack";

export const NODE_REFERENCE_STORED_PROPERTIES = [
  "id",
  "type",
  "space_id",
  "definition_id",
  "snapshot_id",
].map((name) => NodeReference.property(name));

/** Maps a builtin NodeType to a PostgresTable. */
export function mapBuiltinNodeToDatabaseTable(nodeType: NodeType): PostgresTable {
  const nodeClass = NODE_CLASS_BY_TYPE[nodeType];
  const tableName = `${POSTGRES_BUILTIN_TABLE_PREFIX}${nodeType}`;
  const columns: PostgresColumn[] = [];
  const constraints: PostgresConstraint[] = [];
  const indexes: PostgresIndex[] = [];

  // get stored properties and sort by id
  const properties = Object.values(nodeClass.__definition__.properties).filter((p) => p.isStored);
  properties.sort((a, b) => (a.id || -1) - (b.id || -1));

  // map properties to columns, add per-column indices
  for (const prop of properties) {
    if (prop.scalarType === ScalarType.NODE_REFERENCE) {
      // unravel node ptr column
      if (prop.cardinality !== TypeCardinality.SCALAR) {
        throw new Error(`non-scalar node reference: ${prop}`);
      }
      for (const unraveledProp of NODE_REFERENCE_STORED_PROPERTIES) {
        if (!unraveledProp.primitiveType) {
          throw new Error(`undetermined type for ${unraveledProp}`);
        }
        const column = new PostgresColumn({
          name: `${prop.id}_${unraveledProp.id}`,
          type: unraveledProp.primitiveType,
          prop: unraveledProp,
          isNullable: !unraveledProp.isRequired,
        });
        columns.push(column);
      }
    } else {
      // regular column
      if (!prop.primitiveType) {
        throw new Error(`undetermined type for ${prop}`);
      }

      const column = new PostgresColumn({
        name: String(prop.id),
        type: prop.primitiveType,
        prop: prop,
        isArray: prop.cardinality === TypeCardinality.LIST,
        isNullable: !prop.isRequired,
        isPrimaryKey: prop.name === "id",
      });
      columns.push(column);
    }

    // unique
    if (prop.isUnique) {
      const index = new PostgresIndex({
        innerName: `unique_${prop.id}`,
        type: PostgresIndexType.BTREE,
        columns: [String(prop.id)],
        isUnique: true,
      });
      indexes.push(index);
    }
  }

  // extras
  for (const index of nodeClass.__definition__.indexes) {
    if (index.name && index.name.startsWith("destack_")) {
      throw new Error(`bad index name: ${index}`);
    }
    const extraIndex = PostgresIndex.fromIndex(index);
    indexes.push(extraIndex);
  }

  return new PostgresTable({
    name: tableName,
    nodeType: nodeType,
    columns: columns,
    constraints: constraints,
    indexes: indexes,
  });
}

/** Gets the builtin schema for the given Stores. */
export function getBuiltinSchema(...storeKeys: StoreKey[]): PostgresSchema {
  const nodeTypes: NodeType[] = [];
  for (const nodeClass of Object.values(NODE_CLASS_BY_TYPE)) {
    if (
      storeKeys.some((storeKey) => nodeClass.__definition__.primaryStoreKeys.includes(storeKey))
    ) {
      nodeTypes.push(nodeClass.metatype);
    }
  }
  const tables: PostgresTable[] = [];
  for (const nodeType of nodeTypes) {
    tables.push(mapBuiltinNodeToDatabaseTable(nodeType));
  }
  return new PostgresSchema({ tables, extensions: EXTENSIONS });
}
