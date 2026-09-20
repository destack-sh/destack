/** Failures reported by database preparation and connection lifetimes. */
export type DatabaseErrorCode =
    | "MIGRATION_HISTORY"
    | "INVALID_MIGRATION"
    | "MIGRATION_FAILED"
    | "CONNECTION_CLOSED"
    | "TRANSACTION_CLOSED";

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
    throw new TypeError(`Unhandled database variant: ${String(value)}.`);
}
