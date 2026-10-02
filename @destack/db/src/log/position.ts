import { sql, type SQL } from "../sql/index.ts";
import { defineSchema, schema } from "@destack/schema";
import type { Dialect } from "../dialect/dialect.ts";
import { LOG, LOG_EPOCH, LOG_HORIZON, LOG_TRANSACTION } from "./schema.ts";

/** A log sequence or time as drivers return it, read as a safe integer: PostgreSQL returns its bigints as text. */
export const LogInteger = schema
    .union([schema.number(), schema.string(), schema.bigint()])
    .transform((value) => Number(value))
    .pipe(schema.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER));

/** The schema of a position in the log. */
const logPositionSchema = defineSchema(
    schema.object({
        /** The log's epoch, renewed when the database is restored. */
        epoch: schema.string().min(1),
        /** The log sequence within the epoch. */
        sequence: schema.number().int().nonnegative(),
    }),
);
/** A position in the log: an epoch and a sequence within it. */
export type LogPosition = schema.Infer<typeof logPositionSchema>;

/** A position in the log: an epoch and a sequence within it. */
export const LogPosition = Object.assign(logPositionSchema, {
    /** Report whether a position is later than another. */
    isAfter(position: LogPosition, other: LogPosition): boolean {
        return position.epoch === other.epoch
            ? position.sequence > other.sequence
            : position.epoch > other.epoch;
    },
});

/** The log's head as selection fields: its epoch, latest logged sequence and horizon, each read once per statement. */
export function headFields(dialect: Dialect): {
    readonly epoch: SQL<string>;
    readonly logged: SQL<number | null>;
    readonly horizon: SQL<number>;
} {
    // read SQLite's latest entry outside the open transaction
    const log = sql.identifier(LOG);
    const logged =
        dialect === "sqlite"
            ? sql`(SELECT sequence FROM ${log} AS entry
                WHERE NOT EXISTS (SELECT 1 FROM ${sql.identifier(LOG_TRANSACTION)} AS marker
                    WHERE marker.slot = 1 AND marker.id = entry."transaction")
                ORDER BY sequence DESC LIMIT 1)`
            : sql`(SELECT max(sequence) FROM ${log})`;

    return {
        epoch: sql`(SELECT epoch FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1)`.mapWith(
            (value) => schema.string().parse(value),
        ),
        logged: logged.mapWith((value) => LogInteger.nullable().parse(value)),
        horizon: sql`(SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1)`.mapWith(
            (value) => LogInteger.parse(value),
        ),
    };
}

/**
 * Select the log's head in one row: its epoch, latest sequence and horizon.
 *
 * Inside a transaction the head leaves out the transaction's own entries.
 */
export function selectHead(dialect: Dialect): SQL {
    const head = headFields(dialect);

    return sql`SELECT ${head.epoch} AS epoch, ${head.logged} AS logged, ${head.horizon} AS horizon`;
}

/** Read a head's latest sequence: the latest logged one, or the horizon. */
export function latestOf(logged: number | null, horizon: number): number {
    return logged === null ? horizon : Math.max(logged, horizon);
}
