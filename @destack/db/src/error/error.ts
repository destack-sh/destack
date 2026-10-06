import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each database failure: a conflict, a bad request, a lost history, or the host's. */
const SERVICE_CODES = {
    INVALID_MIGRATION: "INTERNAL_SERVER_ERROR",
    MIGRATION_FAILED: "INTERNAL_SERVER_ERROR",
    PLAN_CHANGED: "INTERNAL_SERVER_ERROR",
    NOT_APPLIED: "INTERNAL_SERVER_ERROR",
    CONNECTION_CLOSED: "SERVICE_UNAVAILABLE",
    OWNER_CHANGED: "SERVICE_UNAVAILABLE",
    TRANSACTION_CLOSED: "INTERNAL_SERVER_ERROR",
    TRANSACTION_REQUIRED: "INTERNAL_SERVER_ERROR",
    TREE_NOT_FOUND: "INTERNAL_SERVER_ERROR",
    CHANGES_COMPACTED: "GONE",
    STALE_EPOCH: "STALE_EPOCH",
    CONCURRENT_UPDATE: "SERVICE_UNAVAILABLE",
    DUPLICATE: "CONFLICT",
    BROKEN_REFERENCE: "CONFLICT",
    INVALID_RECORD: "BAD_REQUEST",
    INVALID_QUERY: "BAD_REQUEST",
    QUERY_FAILED: "INTERNAL_SERVER_ERROR",
    NO_CHANNEL: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** The database failure codes. */
export type DatabaseErrorCode = keyof typeof SERVICE_CODES;

/** A database failure with a stable code. */
export class DatabaseError extends Error implements DomainError {
    /** The failure code. */
    readonly code: DatabaseErrorCode;

    /** Create a database failure. */
    constructor(code: DatabaseErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "DatabaseError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
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
    if (
        code === "40001" ||
        code === "40P01" ||
        error.message.includes("database is locked") ||
        error.message.includes("database table is locked")
    ) {
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
