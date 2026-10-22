import { schema } from "@destack/schema";
import {
    type DomainError,
    CALL_ERROR_STATUSES,
    type CallErrorCode,
    type Failure,
} from "@destack/error";

/** Failures during source inspection and compilation. */
export const BuildErrorCode = schema.enum(["INSPECTION_FAILED", "BUILD_FAILED"]);

/** A machine-readable build failure code. */
export type BuildErrorCode = schema.Infer<typeof BuildErrorCode>;

/** The call error code of each build failure: source that cannot be built. */
const CALL_CODES: Readonly<Record<BuildErrorCode, CallErrorCode>> = {
    INSPECTION_FAILED: "UNPROCESSABLE_CONTENT",
    BUILD_FAILED: "UNPROCESSABLE_CONTENT",
};

/** The errors a Rolldown bundle failure aggregates. */
const BundleErrors = schema.array(schema.looseObject({ message: schema.string() })).min(1);

/** A build failure with its original cause. */
export class BuildError extends Error implements DomainError {
    /** The stable failure code. */
    readonly code: BuildErrorCode;

    /** Create a build failure. */
    constructor(code: BuildErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "BuildError";
        this.code = code;
    }

    /** The HTTP status of the call error a caller receives. */
    get status(): number {
        return CALL_ERROR_STATUSES[CALL_CODES[this.code]];
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }

    /** Report any failure as a build failure, keeping a build failure as it is. */
    static from(cause: unknown): BuildError {
        // keep build failures as they are
        if (cause instanceof BuildError) {
            return cause;
        }

        return BuildError.fromBundle(cause);
    }

    /** Report a failed bundle by its error messages, keeping the bundler's error as cause. */
    static fromBundle(error: unknown): BuildError {
        // read the errors a Rolldown bundle failure aggregates
        const errors = BundleErrors.safeParse(
            error instanceof Error && "errors" in error ? error.errors : undefined,
        );
        if (!errors.success) {
            const message = error instanceof Error ? error.message : String(error);

            return new BuildError("BUILD_FAILED", message, { cause: error });
        }
        const messages = errors.data.map((entry) => entry.message);

        return new BuildError("BUILD_FAILED", messages.join("\n"), { cause: error });
    }
}

/** Report whether a filesystem failure is a missing file. */
export function isMissing(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}
