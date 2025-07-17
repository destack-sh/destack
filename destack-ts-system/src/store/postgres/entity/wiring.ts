import {
  assertNever,
  base64Decode,
  base64Encode,
  timedeltaFromISOFormat,
  timedeltaToISOFormat,
} from "@destack/utils";
import { PostgresTable } from "@desys/store/postgres/entity/core";
import { NODE_REFERENCE_STORED_PROPERTIES } from "@desys/store/postgres/entity/map";
import {
  Node,
  NODE_CLASS_BY_TYPE,
  NODE_REFERENCE_ID_KEY,
  NODE_TYPE_SCALAR_BY_TYPE,
  NodeReference,
  PrimitiveType,
  ScalarType,
  StructType,
  Type,
  TypeCardinality,
  Value,
} from "destack";

/** Pack a Node value into a row of columns. */
export function packNodeRow(options: { table: PostgresTable; value: Value }): Record<string, any> {
  const { table, value } = options;
  if (value.type.nodeType == null) {
    throw new Error(`no node type for ${value.repr()}`);
  }
  const nodeClass = NODE_CLASS_BY_TYPE[value.type.nodeType];

  // get stored properties and sort by id
  const properties = Object.values(nodeClass.__definition__.properties).filter((p) => p.isStored);
  properties.sort((a, b) => a.id - b.id);

  // pack properties
  const row: Record<string, any> = {};
  for (const prop of properties) {
    packColumnWide({
      type: prop.toType(),
      value: value.value[String(prop.id)],
      table,
      columnName: String(prop.id),
      columnOut: row,
    });
  }

  return row;
}

/**
 * Unpack a row of columns into a Node value.
 */
export function unpackNodeRow(options: { table: PostgresTable; row: Record<string, any> }): {
  value: Value;
  ptr: NodeReference;
} {
  const { table, row } = options;
  const nodeType = table.nodeType;
  const nodeClass = NODE_CLASS_BY_TYPE[nodeType];

  // get stored properties and sort by id
  const properties = Object.values(nodeClass.__definition__.properties).filter((p) => p.isStored);
  properties.sort((a, b) => a.id - b.id);

  // unpack properties
  const nodeValue: Record<string, any> = { "1": table.nodeType };
  for (const prop of properties) {
    const propUnpacked = unpackColumnWide({
      type: prop.toType(),
      row,
      table,
      columnName: String(prop.id),
    });
    if (propUnpacked != null) {
      nodeValue[String(prop.id)] = propUnpacked;
    }
  }

  const type = NODE_TYPE_SCALAR_BY_TYPE[nodeType];
  const value = new Value({ type, value: nodeValue });
  const nodePtr = new NodeReference({
    type: nodeType,
    id: nodeValue[String(Node.property("id").id)],
  });

  return { value: value, ptr: nodePtr };
}

/** Pack a dynamic column value into a single column value. */
function _packColumnScalar(type: Type, value: any): any {
  if (type.scalarType === ScalarType.NODE_REFERENCE) {
    throw new Error(`unhandled node ref: ${type.repr()}`);
  }

  if (type.scalarType === ScalarType.PRIMITIVE) {
    if (type.primitiveType === PrimitiveType.BYTES) {
      return base64Decode(value);
    } else if (type.primitiveType === PrimitiveType.UUID) {
      return value;
    } else if (type.primitiveType === PrimitiveType.DATETIME) {
      return value;
    } else if (type.primitiveType === PrimitiveType.DATE) {
      return value;
    } else if (type.primitiveType === PrimitiveType.TIME) {
      return value;
    } else if (type.primitiveType === PrimitiveType.DURATION) {
      return timedeltaFromISOFormat(value);
    } else {
      return value;
    }
  } else if (type.scalarType === ScalarType.ENUM) {
    return value;
  } else if (type.scalarType === ScalarType.NODE_VALUE || type.scalarType === ScalarType.STRUCT) {
    return JSON.stringify(value);
  } else {
    assertNever(type.scalarType);
  }
}

/** Unpack a dynamic column value from a single column value. */
function _unpackColumnScalar(type: Type, value: any): any {
  if (type.scalarType === ScalarType.NODE_REFERENCE) {
    throw new Error(`unhandled node ref: ${type.repr()}`);
  }

  if (type.scalarType === ScalarType.PRIMITIVE) {
    if (type.primitiveType === PrimitiveType.BYTES) {
      return base64Encode(value);
    } else if (type.primitiveType === PrimitiveType.UUID) {
      return String(value);
    } else if (type.primitiveType === PrimitiveType.DATETIME) {
      return (value as Date).toISOString();
    } else if (type.primitiveType === PrimitiveType.DATE) {
      return (value as Date).toISOString();
    } else if (type.primitiveType === PrimitiveType.TIME) {
      return (value as Date).toISOString();
    } else if (type.primitiveType === PrimitiveType.DURATION) {
      return timedeltaToISOFormat(value);
    } else {
      return value;
    }
  } else if (type.scalarType === ScalarType.ENUM) {
    return value;
  } else if (type.scalarType === ScalarType.NODE_VALUE || type.scalarType === ScalarType.STRUCT) {
    return JSON.parse(value);
  } else {
    assertNever(type.scalarType);
  }
}

/** Pack a dynamic column value into a single column value. */
export function packColumnFlat(options: { type: Type; value: any }): any {
  const { type, value } = options;

  if (type.cardinality === TypeCardinality.SCALAR) {
    return _packColumnScalar(type, value);
  } else if (type.cardinality === TypeCardinality.LIST) {
    return value.map((v: any) => _packColumnScalar(type, v));
  } else if (type.cardinality === TypeCardinality.MAP) {
    return JSON.stringify(value);
  } else {
    assertNever(type.cardinality);
  }
}

/** Pack a dynamic column value into all of its columns. */
export function packColumnWide(options: {
  type: Type;
  value: any;
  table: PostgresTable;
  columnName: string;
  columnOut: Record<string, any>;
}): void {
  const { type, value, table, columnName, columnOut } = options;

  if (type.cardinality === TypeCardinality.SCALAR) {
    if (type.scalarType === ScalarType.NODE_REFERENCE) {
      // node references fan out to multiple columns
      for (const unraveledProp of NODE_REFERENCE_STORED_PROPERTIES) {
        const unraveledColumnName = `${columnName}_${unraveledProp.id}`;
        const unraveledValuePacked =
          value != null
            ? _packColumnScalar(unraveledProp.toType(), value[String(unraveledProp.id)])
            : null;
        columnOut[unraveledColumnName] = unraveledValuePacked;
      }
    } else {
      const valuePacked = value != null ? _packColumnScalar(type, value) : null;
      columnOut[columnName] = valuePacked;
    }
  } else if (type.cardinality === TypeCardinality.LIST) {
    columnOut[columnName] = value ? value.map((v: any) => _packColumnScalar(type, v)) : [];
  } else if (type.cardinality === TypeCardinality.MAP) {
    columnOut[columnName] = value != null ? JSON.stringify(value) : null;
  } else {
    assertNever(type.cardinality);
  }
}

/** Unpack a dynamic column value from a single column value. */
export function unpackColumn(options: { type: Type; value: any }): any {
  const { type, value } = options;

  if (type.cardinality === TypeCardinality.SCALAR) {
    return _unpackColumnScalar(type, value);
  } else if (type.cardinality === TypeCardinality.LIST) {
    return value.map((v: any) => _unpackColumnScalar(type, v));
  } else if (type.cardinality === TypeCardinality.MAP) {
    return JSON.parse(value);
  } else {
    assertNever(type.cardinality);
  }
}

/**  */
export function unpackColumnWide(options: {
  type: Type;
  row: Record<string, any>;
  table: PostgresTable;
  columnName: string;
}): any | undefined {
  const { type, row, table, columnName } = options;
  if (type.scalarType === ScalarType.NODE_REFERENCE) {
    if (row[`${columnName}_${NODE_REFERENCE_ID_KEY}`] != null) {
      const nodeRefValue: Record<string, any> = {
        "1": StructType.NODE_REFERENCE,
      };
      for (const unraveledProp of NODE_REFERENCE_STORED_PROPERTIES) {
        nodeRefValue[String(unraveledProp.id)] = row[`${columnName}_${unraveledProp.id}`];
      }
      return nodeRefValue;
    } else {
      return undefined;
    }
  } else {
    if (row[columnName] != null) {
      return _unpackColumnScalar(type, row[columnName]);
    } else {
      return undefined;
    }
  }
}
