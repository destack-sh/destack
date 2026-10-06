import { defineSchema, schema } from "@destack/schema";
import {
    type ReportableError,
    SERVICE_ERROR_STATUSES,
    type ServiceErrorCode,
    type ServiceErrorReport,
} from "@destack/error";

/** Failures during source inspection and compilation. */
export const BuildErrorCode = defineSchema(schema.enum(["INSPECTION_FAILED", "BUILD_FAILED"]));

/** A machine-readable build failure code. */
export type BuildErrorCode = schema.Infer<typeof BuildErrorCode>;

/** The service error code of each build failure: source that cannot be built. */
const SERVICE_CODES: Readonly<Record<BuildErrorCode, ServiceErrorCode>> = {
    INSPECTION_FAILED: "UNPROCESSABLE_CONTENT",
    BUILD_FAILED: "UNPROCESSABLE_CONTENT",
};

/** The errors a Rolldown bundle failure aggregates. */
const BundleErrors = schema.array(schema.looseObject({ message: schema.string() })).min(1);

/** A build failure with its original cause. */
export class BuildError extends Error implements ReportableError {
    /** The stable failure code. */
    readonly code: BuildErrorCode;

    /** Create a build failure. */
    constructor(code: BuildErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "BuildError";
        this.code = code;
    }

    /** The HTTP status of the service error a caller receives. */
    get status(): number {
        return SERVICE_ERROR_STATUSES[SERVICE_CODES[this.code]];
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
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

/** Report whether a filesystem failure names a missing file. */
export function isMissing(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}
