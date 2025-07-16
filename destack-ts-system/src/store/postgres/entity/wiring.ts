import { PostgresTable } from "@desys/store/postgres/entity/core";
import { Type, Value } from "destack";

export function packColumnFlat(options: { type: Type; value: any }): any {
  throw new Error("not implemented");
}

export function packColumnWide(options: {
  type: Type;
  value: any;
  table: PostgresTable;
  columnName: string;
  columnOut: Record<string, any>;
}): void {
  throw new Error("not implemented");
}

export function unpackColumn(options: { type: Type; value: any }): JSON {
  throw new Error("not implemented");
}

export function packNodeRow(options: {
  table: PostgresTable;
  value: Value;
}): Array<any> {
  throw new Error("not implemented");
}

export function unpackNodeRow(options: {
  table: PostgresTable;
  row: Record<string, any>;
}): Value {
  throw new Error("not implemented");
}
