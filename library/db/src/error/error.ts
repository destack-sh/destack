/** Failures reported when preparing a database schema. */
export type DatabaseErrorCode = "MIGRATION_HISTORY" | "INVALID_MIGRATION" | "MIGRATION_FAILED";

/** A database preparation failure with a stable code. */
export class DatabaseError extends Error {
    /** The failure code. */
    readonly code: DatabaseErrorCode;

    /** Create a database preparation failure. */
    constructor(code: DatabaseErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "DatabaseError";
        this.code = code;
    }
}
