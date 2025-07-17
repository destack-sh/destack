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
  NODE_CLASS_BY_TYPE,
  NODE_TYPE_SCALAR_BY_TYPE,
  PrimitiveType,
  ScalarType,
  Type,
  TypeCardinality,
  Value,
} from "destack";
import { Temporal } from "temporal-polyfill";

/** Pack a Node value into a row of columns. */
export function packNodeRow(options: { table: PostgresTable; value: Value }): Array<any> {
  const { table, value } = options;
  if (value.type.nodeType == null) {
    throw new Error(`no node type for ${value.repr()}`);
  }
  const nodeClass = NODE_CLASS_BY_TYPE[value.type.nodeType];

  // get stored properties and sort by id
  const properties = Object.values(nodeClass.__definition__.properties).filter((p) => p.isStored);
  properties.sort((a, b) => a.id - b.id);

  // pack properties
  const row: Map<string, any> = new Map();
  for (const prop of properties) {
    packColumnWide({
      type: prop.toType(),
      value: value.value[String(prop.id)],
      table,
      columnName: String(prop.id),
      columnOut: row,
    });
  }
  console.log("packNodeRow", table.name, row);
  return Array.from(row.values());
}

/**
 * Unpack a row of columns into a Node value.
 */
export function unpackNodeRow(options: { table: PostgresTable; row: Record<string, any> }): Value {
  const { table, row } = options;
  const nodeType = table.nodeType;
  const nodeClass = NODE_CLASS_BY_TYPE[nodeType];

  // get stored properties and sort by id
  const properties = Object.values(nodeClass.__definition__.properties).filter((p) => p.isStored);
  properties.sort((a, b) => a.id - b.id);

  // unpack properties
  const value: Record<string, any> = {};
  for (const prop of properties) {
    const unpackedProp = unpackColumn({ type: prop.toType(), value: row.get(String(prop.id)) });
    value[String(prop.id)] = unpackedProp;
  }

  const type = NODE_TYPE_SCALAR_BY_TYPE[nodeType];
  return new Value({ type, value });
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
      return Temporal.Instant.from(value).toZonedDateTimeISO("UTC");
    } else if (type.primitiveType === PrimitiveType.DATE) {
      return Temporal.PlainDate.from(value);
    } else if (type.primitiveType === PrimitiveType.TIME) {
      return Temporal.PlainTime.from(value);
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
      return value.toString({ timeZoneName: "never" });
    } else if (type.primitiveType === PrimitiveType.DATE) {
      return value.toString();
    } else if (type.primitiveType === PrimitiveType.TIME) {
      return value.toString();
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
    return value ? value.map((v: any) => _packColumnScalar(type, v)) : [];
  } else if (type.cardinality === TypeCardinality.MAP) {
    return JSON.stringify(value || {});
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
  columnOut: Map<string, any>;
}): void {
  const { type, value, table, columnName, columnOut } = options;

  if (type.cardinality === TypeCardinality.SCALAR) {
    if (type.scalarType === ScalarType.NODE_REFERENCE) {
      // node references fan out to multiple columns
      for (const unraveledProp of NODE_REFERENCE_STORED_PROPERTIES) {
        const unraveledColumnName = `${columnName}_${unraveledProp.id}`;
        columnOut.set(
          unraveledColumnName,
          value != null
            ? _packColumnScalar(unraveledProp.toType(), value[String(unraveledProp.id)])
            : null,
        );
      }
    } else {
      const valuePacked = value != null ? _packColumnScalar(type, value) : null;
      columnOut.set(columnName, valuePacked);
    }
  } else if (type.cardinality === TypeCardinality.LIST) {
    columnOut.set(columnName, value ? value.map((v: any) => _packColumnScalar(type, v)) : []);
  } else if (type.cardinality === TypeCardinality.MAP) {
    columnOut.set(columnName, JSON.stringify(value || {}));
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
    return value ? value.map((v: any) => _unpackColumnScalar(type, v)) : [];
  } else if (type.cardinality === TypeCardinality.MAP) {
    return JSON.parse(value);
  } else {
    assertNever(type.cardinality);
  }
}
