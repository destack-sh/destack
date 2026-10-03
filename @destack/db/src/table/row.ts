import { schema } from "@destack/schema";
import { ColumnValue } from "./column.ts";

/** A row by column property. */
export type Row = Readonly<Record<string, ColumnValue>>;
/** A row by column property. */
export const Row: schema.Schema<Row> = schema.record(schema.string(), ColumnValue);
