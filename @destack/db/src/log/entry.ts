import type { Dialect } from "../dialect/dialect.ts";
import { DatabaseError } from "../error/error.ts";
import { LogColumn } from "./position.ts";

/** One committed change as the log stores it. */
export interface LogEntry {
    /** The entry's sequence, null for PostgreSQL's open transaction until it commits. */
    readonly sequence: number | null;
    /** The committing transaction. */
    readonly transaction: string | null;
    /** The changed table's SQL name. */
    readonly table: string;
    /** The encoded primary key, in key order. */
    readonly key: readonly unknown[];
    /** Insert, update or delete. */
    readonly operation: "insert" | "update" | "delete";
    /** The encoded row by column name, which its columns decode. */
    readonly row: Readonly<Record<string, unknown>>;
    /** The values an update changed, before it, by column name. */
    readonly previous: Readonly<Record<string, unknown>> | null;
    /** The changed row's scope. */
    readonly scope: string;
    /** The change time. */
    readonly changedAt: number;
}

/** The log's change entries. */
export const LogEntry = {
    /** Read an entry by its columns' driver types, since the log writes every entry itself. */
    read(row: Readonly<Record<string, unknown>>, dialect: Dialect): LogEntry {
        const previous = row["previous"];

        return {
            sequence: LogColumn.nullableInteger(row["sequence"]),
            transaction: LogColumn.nullableText(row["transaction"]),
            table: LogColumn.text(row["table"]),
            key: LogColumn.array(row["key"], dialect),
            operation: LogEntry.operation(row["operation"]),
            row: LogColumn.object(row["row"], dialect),
            previous: previous === null ? null : LogColumn.object(previous, dialect),
            scope: LogColumn.text(row["scope"]),
            changedAt: LogColumn.integer(row["changed_at"]),
        };
    },

    /** Read an entry's operation, failing for any other value. */
    operation(value: unknown): LogEntry["operation"] {
        if (value !== "insert" && value !== "update" && value !== "delete") {
            throw DatabaseError.of(
                "QUERY_BROKEN",
                `expected an operation from the log, got '${String(value)}'`,
            );
        }

        return value;
    },
};

/** A committed entry, its sequence known. */
export type SequencedEntry = LogEntry & { readonly sequence: number };
