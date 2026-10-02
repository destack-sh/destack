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
    | "QUERY_FAILED"
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
    // keep classified and non-driver failures
    if (error instanceof DatabaseError || !(error instanceof Error)) {
        return error;
    }

    // classify by PostgreSQL state or SQLite message
    const code = errorCode(error);
    if (code === "40001" || code === "40P01") {
        return new DatabaseError(
            "CONCURRENT_UPDATE",
            "a concurrent transaction changed the same records",
            {
                cause: error,
            },
        );
    } else if (code === "23505" || error.message.includes("UNIQUE constraint failed")) {
        return new DatabaseError("DUPLICATE", "a record with the same unique key exists", {
            cause: error,
        });
    } else if (
        code === "23503" ||
        code === "23001" ||
        error.message.includes("FOREIGN KEY constraint failed")
    ) {
        return new DatabaseError(
            "BROKEN_REFERENCE",
            "the change would leave a reference to a missing record",
            {
                cause: error,
            },
        );
    } else if (code === "23514" || error.message.includes("CHECK constraint failed")) {
        return new DatabaseError("INVALID_RECORD", "a record fails a declared check", {
            cause: error,
        });
    }

    return error;
}

/** Read a driver failure's code, absent for a failure without one. */
export function errorCode(error: unknown): string | undefined {
    return error instanceof Error && "code" in error && typeof error.code === "string"
        ? error.code
        : undefined;
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
