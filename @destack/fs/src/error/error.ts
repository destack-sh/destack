/** A failed host filesystem operation. */
export class FileSystemError extends Error {
    /** The failed filesystem operation. */
    readonly operation: string;
    /** The affected absolute path. */
    readonly path: string;
    /** The operating-system error identifier. */
    readonly code: string | number;

    /** Retain the operation, path, and operating-system failure. */
    constructor(operation: string, path: string, code: string | number, options?: ErrorOptions) {
        super(`cannot ${operation} ${path}: ${code}`, options);

        this.operation = operation;
        this.path = path;
        this.code = code;

        this.name = "FileSystemError";
    }
}
