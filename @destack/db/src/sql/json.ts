import { defineSchema, schema } from "@destack/schema";

/** Driver values as JSON, keeping signed 64-bit integers and bytes exact. */
export const SQLValue = defineSchema(
    schema.union([
        schema.string(),
        schema.number(),
        schema.boolean(),
        schema.null(),
        schema.object({ integer: schema.string().regex(/^-?(0|[1-9][0-9]*)$(?![\s\S])/u) }),
        schema.object({ bytes: schema.base64() }),
    ]),
);
/** A driver value as JSON carries it. */
export type SQLValue = schema.Infer<typeof SQLValue>;

/** One SQL statement with its positional parameters. */
export const SQLStatement = defineSchema(
    schema.object({
        /** The statement. */
        sql: schema.string().min(1),
        /** The positional parameters, none when absent. */
        parameters: schema.array(SQLValue).exactOptional(),
    }),
);
/** One SQL statement with its positional parameters. */
export type SQLStatement = schema.Infer<typeof SQLStatement>;

/** The rows a statement returned, in column order with duplicate names kept, and the rows it changed. */
export const SQLResult = defineSchema(
    schema.object({
        /** The column names in result order. */
        columns: schema.array(schema.string()),
        /** The cells in matching column order. */
        rows: schema.array(schema.array(SQLValue)),
        /** Whether the server's row or byte limit stopped collecting rows. */
        truncated: schema.boolean(),
        /** The rows the statement inserted, updated or deleted. */
        changed: schema.number().int().min(0),
    }),
);
/** The rows a statement returned and the rows it changed. */
export type SQLResult = schema.Infer<typeof SQLResult>;
