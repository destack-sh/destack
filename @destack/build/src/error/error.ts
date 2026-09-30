import { defineSchema, schema } from "@destack/schema";

/** Failures during source inspection and compilation. */
export const BuildErrorCode = defineSchema(schema.enum(["INSPECTION_FAILED", "BUILD_FAILED"]));

/** A machine-readable build failure code. */
export type BuildErrorCode = schema.Infer<typeof BuildErrorCode>;

/** A build failure with its original cause. */
export class BuildError extends Error {
    /** The stable failure code. */
    readonly code: BuildErrorCode;

    /** Create a build failure. */
    constructor(code: BuildErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "BuildError";
        this.code = code;
    }

    /** Report any failure as a build failure, keeping a build failure as it is. */
    static from(cause: unknown): BuildError {
        // keep build failures as they are
        if (cause instanceof BuildError) {
            return cause;
        }

        return BuildError.fromBundle(cause);
    }

    /** Report a failed bundle by the messages of its errors, keeping the bundler's error as cause. */
    static fromBundle(error: unknown): BuildError {
        // read the errors a Rolldown bundle failure aggregates
        const errors = error instanceof Error && "errors" in error ? error.errors : undefined;
        if (!Array.isArray(errors) || errors.length === 0) {
            const message = error instanceof Error ? error.message : String(error);

            return new BuildError("BUILD_FAILED", message, { cause: error });
        }
        const messages = errors.map((entry: { readonly message: string }) => entry.message);

        return new BuildError("BUILD_FAILED", messages.join("\n"), { cause: error });
    }
}
