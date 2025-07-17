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
  EdgeType,
  NODE_CLASS_BY_TYPE,
  NodeType,
  PrimitiveType,
  ScalarType,
  StoreKey,
  TypeCardinality,
} from "destack";

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
    if (prop.edgeType === EdgeType.PARENT && nodeType === NodeType.SPACE) {
      continue; // no parent for root nodes
    }

    if (typeof prop.id !== "number") {
      throw new Error(`undetermined id for ${prop}`);
    }

    if (prop.scalarType === ScalarType.NODE_REFERENCE) {
      // unravel node ptr column
      if (prop.cardinality !== TypeCardinality.SCALAR) {
        throw new Error(`non-scalar node reference: ${prop}`);
      }

      const column = new PostgresColumn({
        name: `${prop.id}_id`,
        type: PrimitiveType.UUID,
        prop: prop,
        isNullable: !prop.isRequired,
      });
      columns.push(column);
      const nodeTypeColumn = new PostgresColumn({
        name: `${prop.id}_type`,
        type: PrimitiveType.INT32,
        prop: prop,
        isNullable: !prop.isRequired,
      });
      columns.push(nodeTypeColumn);
      const spaceIdColumn = new PostgresColumn({
        name: `${prop.id}_space_id`,
        type: PrimitiveType.UUID,
        prop: prop,
        isNullable: true,
      });
      columns.push(spaceIdColumn);
      const tableIdColumn = new PostgresColumn({
        name: `${prop.id}_definition_id`,
        type: PrimitiveType.UUID,
        prop: prop,
        isNullable: true,
      });
      columns.push(tableIdColumn);
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
