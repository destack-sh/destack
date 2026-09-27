/** Failures reported by database preparation and connection lifetimes. */
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
    | "INVALID_QUERY";

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

/** Reject an unhandled variant and require exhaustive branching at compile time. */
export function assertNever(value: never): never {
    throw new TypeError(`unhandled database variant: ${String(value)}`);
}

/** Classify a failed statement: a transaction that lost to a concurrent one, a row duplicating a unique key, a change breaking a reference, or a row failing a check. */
export function classifyError(error: unknown): unknown {
    // keep a classified cause, else classify by the PostgreSQL state or SQLite message among the causes
    for (let cause = error; cause instanceof Error; cause = cause.cause) {
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
        } else if (code === "23503" || cause.message.includes("FOREIGN KEY constraint failed")) {
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

/** Report whether an error is a failure or wraps it among its causes. */
export function wraps(error: unknown, failure: unknown): boolean {
    for (let current = error; current !== undefined;) {
        if (current === failure) {
            return true;
        }
        current = current instanceof Error ? current.cause : undefined;
    }

    return false;
}
