import { defineSchema, schema } from "@destack/schema";
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

/** A package failure with its original cause. */
export class PackageError extends Error {
    /** Stable failure code. */
    readonly code: PackageErrorCode;

    /** Create a package error with its code, message and cause. */
    constructor(code: PackageErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "PackageError";
        this.code = code;
    }
}

/** Why a host cannot grant a capability: it has nothing of its kind to grant, or cannot enforce it as declared. */
export type CapabilityErrorCode = "UNSUPPORTED" | "UNENFORCEABLE";

/** A workload's capability its host cannot grant, refused before the workload starts. */
export class CapabilityError extends Error {
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
}
