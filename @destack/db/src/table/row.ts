import type { ColumnValue } from "./column.ts";

/** A row by column property. */
export type Row = Readonly<Record<string, ColumnValue>>;
