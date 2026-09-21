/** A failed host filesystem operation. */
export class FileSystemError extends Error {
    /** Retain the operation, path, and operating-system failure. */
    constructor(
        /** The failed filesystem operation. */
        readonly operation: string,
        /** The affected absolute path. */
        readonly path: string,
        /** The operating-system error identifier. */
        readonly code: string | number,
        options?: ErrorOptions,
    ) {
        super(`cannot ${operation} ${path}: ${code}`, options);
        this.name = "FileSystemError";
    }
}
