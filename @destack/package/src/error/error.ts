import {
    defineSchema,
    type ReportableError,
    schema,
    type ServiceErrorCode,
    type ServiceErrorReport,
} from "@destack/schema";
import type { CapabilityName } from "../definition/capability.ts";

/** Invalid package definitions and distributed files. */
export const PackageErrorCode = defineSchema(
    schema.enum([
        "INVALID_DEFINITION",
        "UNSUPPORTED_LANGUAGE",
        "UNSUPPORTED_TARGET",
        "INVALID_EXPORT",
        "INVALID_DEPENDENCY",
        "INVALID_FILE",
        "INVALID_INSPECTION",
    ]),
);

/** A machine-readable package failure code. */
export type PackageErrorCode = schema.Infer<typeof PackageErrorCode>;

/** The service error code of each package failure: an invalid package, or one no host processes. */
const SERVICE_CODES: Readonly<Record<PackageErrorCode, ServiceErrorCode>> = {
    INVALID_DEFINITION: "BAD_REQUEST",
    UNSUPPORTED_LANGUAGE: "UNPROCESSABLE_CONTENT",
    UNSUPPORTED_TARGET: "UNPROCESSABLE_CONTENT",
    INVALID_EXPORT: "BAD_REQUEST",
    INVALID_DEPENDENCY: "BAD_REQUEST",
    INVALID_FILE: "BAD_REQUEST",
    INVALID_INSPECTION: "UNPROCESSABLE_CONTENT",
};

/** A package failure with its original cause. */
export class PackageError extends Error implements ReportableError {
    /** Stable failure code. */
    readonly code: PackageErrorCode;

    /** Create a package error with its code, message and cause. */
    constructor(code: PackageErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "PackageError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}

/** Why a host cannot grant a capability: it has nothing of its kind to grant, or cannot enforce it as declared. */
export type CapabilityErrorCode = "UNSUPPORTED" | "UNENFORCEABLE";

/** A workload's capability its host cannot grant, refused before the workload starts. */
export class CapabilityError extends Error implements ReportableError {
    /** Stable failure code. */
    readonly code: CapabilityErrorCode;
    /** The capability the host cannot grant. */
    readonly capability: CapabilityName;

    /** Create a capability error with its code, the capability and the gap. */
    constructor(
        code: CapabilityErrorCode,
        capability: CapabilityName,
        message: string,
        options?: ErrorOptions,
    ) {
        // keep the code and the capability beside the message
        super(message, options);
        this.name = "CapabilityError";
        this.code = code;
        this.capability = capability;
    }

    /** Report a capability no host grants as a workload the host cannot process. */
    toServiceError(): ServiceErrorReport {
        return { code: "UNPROCESSABLE_CONTENT", message: this.message };
    }
}
