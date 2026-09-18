import { defineSchema, schema } from "@destack/schema";

/** Invalid package definitions and distributed files. */
export const PackageErrorCode = defineSchema(schema.enum([
    "INVALID_DEFINITION",
    "UNSUPPORTED_LANGUAGE",
    "UNSUPPORTED_TARGET",
    "INVALID_EXPORT",
    "INVALID_DEPENDENCY",
    "INVALID_FILE",
    "INVALID_INSPECTION",
]));

/** A machine-readable package failure code. */
export type PackageErrorCode = schema.Infer<typeof PackageErrorCode>;

/** A package failure with its original cause. */
export class PackageError extends Error {
    /** Stable failure code. */
    readonly code: PackageErrorCode;

    constructor(code: PackageErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "PackageError";
        this.code = code;
    }
}
