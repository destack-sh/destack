import { defineSchema, schema } from "@destack/schema";
import { DatabaseError } from "../error/error.ts";
import type { DriverValue } from "../table/column.ts";

/** The schema of driver values as JSON, keeping signed 64-bit integers and bytes exact. */
const sqlValueSchema = defineSchema(
    schema.union([
        schema.string(),
        schema.number(),
        schema.boolean(),
        schema.null(),
        schema.object({ integer: schema.string().regex(/^-?(0|[1-9][0-9]*)$/u) }),
        schema.object({ bytes: schema.base64() }),
    ]),
);
/** A driver value in JSON form. */
export type SQLValue = schema.Infer<typeof sqlValueSchema>;

/** Driver values as JSON, keeping signed 64-bit integers and bytes exact. */
export const SQLValue = Object.assign(sqlValueSchema, {
    /** Write a driver value, or bytes a driver returns, in JSON form. */
    fromDriver(value: unknown): SQLValue {
        // keep scalars JSON writes exactly
        if (
            value === null ||
            typeof value === "string" ||
            typeof value === "number" ||
            typeof value === "boolean"
        ) {
            return value;
        }
        // tag integers and bytes
        else if (typeof value === "bigint") {
            return { integer: value.toString() };
        } else if (value instanceof Uint8Array || value instanceof ArrayBuffer) {
            return { bytes: new Uint8Array(value).toBase64() };
        }

        throw new DatabaseError(
            "INVALID_QUERY",
            `a ${typeof value} has no JSON form: SQL values are text, numbers, booleans, integers, bytes or null`,
        );
    },

    /** Read a driver value from its JSON form. */
    toDriver(value: SQLValue): DriverValue {
        // untag integers and bytes
        if (value !== null && typeof value === "object") {
            return "integer" in value ? BigInt(value.integer) : Uint8Array.fromBase64(value.bytes);
        }

        return value;
    },
});

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
