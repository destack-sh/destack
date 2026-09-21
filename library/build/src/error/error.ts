import { defineSchema, schema } from "@destack/schema";

/** Failures during source inspection and compilation. */
export const BuildErrorCode = defineSchema(schema.enum(["INSPECTION_FAILED", "BUILD_FAILED"]));

/** A machine-readable build failure code. */
export type BuildErrorCode = schema.Infer<typeof BuildErrorCode>;

/** A build failure with its original cause. */
export class BuildError extends Error {
    /** Stable failure code. */
    readonly code: BuildErrorCode;

    /** Create a build failure. */
    constructor(code: BuildErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "BuildError";
        this.code = code;
    }
}
