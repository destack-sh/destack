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
    | "DUPLICATE";

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

/** Classify a failed statement: a transaction that lost to a concurrent one, or a row duplicating a unique key. */
export function classifyError(error: unknown): unknown {
    // find the failure's PostgreSQL state or SQLite message among its causes
    for (let cause = error; cause instanceof Error; cause = cause.cause) {
        if (cause instanceof DatabaseError) {
            return error;
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
