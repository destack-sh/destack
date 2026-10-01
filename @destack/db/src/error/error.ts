import { DrizzleQueryError } from "drizzle-orm/errors";

/** The database failure codes. */
export type DatabaseErrorCode =
    | "INVALID_MIGRATION"
    | "MIGRATION_FAILED"
    | "PLAN_CHANGED"
    | "NOT_APPLIED"
    | "CONNECTION_CLOSED"
    | "OWNER_CHANGED"
    | "TRANSACTION_CLOSED"
    | "TRANSACTION_REQUIRED"
    | "TREE_NOT_FOUND"
    | "CHANGES_COMPACTED"
    | "STALE_EPOCH"
    | "CONCURRENT_UPDATE"
    | "DUPLICATE"
    | "BROKEN_REFERENCE"
    | "INVALID_RECORD"
    | "INVALID_QUERY"
    | "INVALID_BLOB"
    | "NO_CHANNEL";

/** A database failure with a stable code. */
export class DatabaseError extends Error {
    /** The failure code. */
    readonly code: DatabaseErrorCode;

    /** Create a database failure. */
    constructor(code: DatabaseErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "DatabaseError";
        this.code = code;
    }
}

/** Reject an unhandled variant. */
export function assertNever(value: never): never {
    throw new TypeError(`unhandled database variant: ${String(value)}`);
}

/** Classify a failed statement as a conflict, duplicate, broken reference or failed check. */
export function classifyError(error: unknown): unknown {
    // classify by PostgreSQL state or SQLite message, looking only through query wrappers
    for (
        let cause = error;
        cause instanceof Error;
        cause = cause instanceof DrizzleQueryError ? cause.cause : undefined
    ) {
        if (cause instanceof DatabaseError) {
            return cause;
        }
        const code = "code" in cause ? cause.code : undefined;
        if (code === "40001" || code === "40P01") {
            return new DatabaseError(
                "CONCURRENT_UPDATE",
                "a concurrent transaction changed the same records",
                { cause: error },
            );
        } else if (code === "23505" || cause.message.includes("UNIQUE constraint failed")) {
            return new DatabaseError("DUPLICATE", "a record with the same unique key exists", {
                cause: error,
            });
        } else if (
            code === "23503" ||
            code === "23001" ||
            cause.message.includes("FOREIGN KEY constraint failed")
        ) {
            return new DatabaseError(
                "BROKEN_REFERENCE",
                "the change would leave a reference to a missing record",
                { cause: error },
            );
        } else if (code === "23514" || cause.message.includes("CHECK constraint failed")) {
            return new DatabaseError("INVALID_RECORD", "a record fails a declared check", {
                cause: error,
            });
        }
    }

    return error;
}

/** Report whether an error is or wraps a failure. */
export function wraps(error: unknown, failure: unknown): boolean {
    for (let current = error; current !== undefined;) {
        if (current === failure) {
            return true;
        }
        current = current instanceof Error ? current.cause : undefined;
    }

    return false;
}
