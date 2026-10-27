import { sql, type SQL } from "../sql/index.ts";
import { schema } from "@destack/schema";
import type { Dialect } from "../dialect/dialect.ts";
import { DatabaseError } from "../error/error.ts";
import { LOG, LOG_EPOCH, LOG_HORIZON, LOG_TRANSACTION } from "./schema.ts";

/** The log's columns, read by the driver types they return, since the log writes them all. */
export const LogColumn = {
    /** Read a sequence or time from a number, big integer or numeric text, failing for others. */
    integer(value: unknown): number {
        // read a number, or a big integer PostgreSQL returns as text
        const number =
            typeof value === "number"
                ? value
                : typeof value === "string" || typeof value === "bigint"
                  ? Number(value)
                  : Number.NaN;
        if (!Number.isSafeInteger(number) || number < 0) {
            throw DatabaseError.of(
                "QUERY_FAILED",
                `expected an integer from the log, got ${String(value)}`,
            );
        }

        return number;
    },

    /** Read a sequence or time, null for SQL NULL. */
    nullableInteger(value: unknown): number | null {
        return value === null ? null : LogColumn.integer(value);
    },

    /** Read text, failing for any other value. */
    text(value: unknown): string {
        if (typeof value !== "string") {
            throw DatabaseError.of(
                "QUERY_FAILED",
                `expected text from the log, got ${String(value)}`,
            );
        }

        return value;
    },

    /** Read text, null for SQL NULL. */
    nullableText(value: unknown): string | null {
        return value === null ? null : LogColumn.text(value);
    },

    /** Read a JSON array: SQLite's text, which it parses, or PostgreSQL's parsed value. */
    array(value: unknown, dialect: Dialect): readonly unknown[] {
        const parsed = parseJson(value, dialect);
        if (!Array.isArray(parsed)) {
            throw DatabaseError.of(
                "QUERY_FAILED",
                `expected an array from the log, got ${String(value)}`,
            );
        }

        return parsed;
    },

    /** Read a JSON object: SQLite's text, which it parses, or PostgreSQL's parsed value. */
    object(value: unknown, dialect: Dialect): Readonly<Record<string, unknown>> {
        const parsed = parseJson(value, dialect);
        if (!isObject(parsed)) {
            throw DatabaseError.of(
                "QUERY_FAILED",
                `expected an object from the log, got ${String(value)}`,
            );
        }

        return parsed;
    },
};

/** The schema of a position in the log. */
const logPositionSchema = schema.object({
    /** The log's identifier, which a restore preserves. */
    log: schema.string().min(1),
    /** The log's epoch, renewed when the database is restored. */
    epoch: schema.string().min(1),
    /** The log sequence within the epoch. */
    sequence: schema.number().int().nonnegative(),
});
/** A position in a log: the log, an epoch and a sequence within it. */
export type LogPosition = schema.Infer<typeof logPositionSchema>;

/** A position in a log: the log, an epoch and a sequence within it. */
export const LogPosition = Object.assign(logPositionSchema, {
    /** Report whether a position is later than another of the same log, refusing another log's. */
    isAfter(position: LogPosition, other: LogPosition): boolean {
        if (position.log !== other.log) {
            throw new TypeError(
                `cannot compare positions of logs ${position.log} and ${other.log}`,
            );
        }

        return position.epoch === other.epoch
            ? position.sequence > other.sequence
            : position.epoch > other.epoch;
    },

    /** Report whether two positions name the same place in the same log. */
    same(position: LogPosition, other: LogPosition): boolean {
        return (
            position.log === other.log &&
            position.epoch === other.epoch &&
            position.sequence === other.sequence
        );
    },
});

/** The log's head as selection fields: log, epoch, latest sequence and horizon, read once each. */
export function headFields(dialect: Dialect): {
    readonly log: SQL<string>;
    readonly epoch: SQL<string>;
    readonly logged: SQL<number | null>;
    readonly horizon: SQL<number>;
} {
    // read SQLite's latest entry outside the open transaction
    const log = sql.relation(LOG);
    const logged =
        dialect === "sqlite"
            ? sql`(SELECT sequence FROM ${log} AS entry WHERE ${committedEntry(dialect, sql`entry`)}
                ORDER BY sequence DESC LIMIT 1)`
            : sql`(SELECT max(sequence) FROM ${log})`;

    return {
        log: sql`(SELECT log FROM ${sql.relation(LOG_EPOCH)} WHERE slot = 1)`.mapWith((value) =>
            LogColumn.text(value),
        ),
        epoch: sql`(SELECT epoch FROM ${sql.relation(LOG_EPOCH)} WHERE slot = 1)`.mapWith((value) =>
            LogColumn.text(value),
        ),
        logged: logged.mapWith((value) => LogColumn.nullableInteger(value)),
        horizon: sql`(SELECT sequence FROM ${sql.relation(LOG_HORIZON)} WHERE slot = 1)`.mapWith(
            (value) => LogColumn.integer(value),
        ),
    };
}

/** Require a log entry to be committed: outside SQLite's open transaction. */
export function committedEntry(dialect: Dialect, entry: SQL): SQL {
    return dialect === "sqlite"
        ? sql`NOT EXISTS (SELECT 1 FROM ${sql.relation(LOG_TRANSACTION)} AS marker
            WHERE marker.slot = 1 AND marker.id = ${entry}."transaction")`
        : sql`TRUE`;
}

/** Select the log's head in one row: its log, epoch, latest sequence and horizon. */
export function selectHead(dialect: Dialect): SQL {
    const head = headFields(dialect);

    return sql`SELECT ${head.log} AS log, ${head.epoch} AS epoch, ${head.logged} AS logged, ${head.horizon} AS horizon`;
}

/** Read a head's latest sequence: the latest logged one, or the horizon. */
export function latestOf(logged: number | null, horizon: number): number {
    return logged === null ? horizon : Math.max(logged, horizon);
}

/** Parse a JSON column: SQLite's text, or PostgreSQL's value its driver parsed. */
function parseJson(value: unknown, dialect: Dialect): unknown {
    return dialect === "sqlite" ? JSON.parse(LogColumn.text(value)) : value;
}

/** Report whether a parsed JSON value is an object. */
function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
