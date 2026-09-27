import { sql, type SQL } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import { LOG, LOG_EPOCH, LOG_HORIZON } from "./schema.ts";

/** A position in the log: an epoch and a sequence within it, where a reader or copy continues. */
export const LogPosition = defineSchema(
    schema.object({
        /** The log's epoch, a time-ordered identifier renewed when the database is restored. */
        epoch: schema.string().min(1),
        /** The log sequence within the epoch. */
        sequence: schema.number().int().nonnegative(),
    }),
);
/** A position in the log: an epoch and a sequence within it, where a reader or copy continues. */
export type LogPosition = schema.Infer<typeof LogPosition>;

/** Report whether a position is later than another: a newer epoch, or a higher sequence within one. */
export function isAfter(position: LogPosition, other: LogPosition): boolean {
    return position.epoch === other.epoch
        ? position.sequence > other.sequence
        : position.epoch > other.epoch;
}

/** Select the log's head in one row: its epoch, its latest logged sequence, and its horizon. */
export function selectHead(): SQL {
    return sql`SELECT epoch, (SELECT max(sequence) FROM ${sql.identifier(LOG)}) AS logged,
            (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
        FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1`;
}

/** Read the latest sequence of a head: the latest logged one, or the horizon once compaction removed every later entry. */
export function latestOf(logged: unknown, horizon: unknown): number {
    const sequences = [logged, horizon].filter((sequence) => sequence !== null).map(Number);

    return sequences.length === 0 ? 0 : Math.max(...sequences);
}
