import { schema } from "@destack/schema";
import { type DomainError, type CallErrorCode, type Failure } from "@destack/error";
import type { SandboxName } from "../definition/sandbox.ts";

/** Invalid package definitions and distributed files. */
export const PackageErrorCode = schema.enum([
    "INVALID_DEFINITION",
    "UNSUPPORTED_LANGUAGE",
    "UNSUPPORTED_TARGET",
    "INVALID_EXPORT",
    "INVALID_DEPENDENCY",
    "INVALID_FILE",
    "INVALID_INSPECTION",
    "INVALID_BUILD",
]);

/** A machine-readable package failure code. */
export type PackageErrorCode = schema.Infer<typeof PackageErrorCode>;

/** The call error code of each package failure: an invalid package, or one no host processes. */
const CALL_CODES: Readonly<Record<PackageErrorCode, CallErrorCode>> = {
    INVALID_DEFINITION: "BAD_REQUEST",
    UNSUPPORTED_LANGUAGE: "UNPROCESSABLE_CONTENT",
    UNSUPPORTED_TARGET: "UNPROCESSABLE_CONTENT",
    INVALID_EXPORT: "BAD_REQUEST",
    INVALID_DEPENDENCY: "BAD_REQUEST",
    INVALID_FILE: "BAD_REQUEST",
    INVALID_INSPECTION: "UNPROCESSABLE_CONTENT",
    INVALID_BUILD: "BAD_REQUEST",
};

/** A package failure with its original cause. */
export class PackageError extends Error implements DomainError {
    /** Stable failure code. */
    readonly code: PackageErrorCode;

    /** Create a package error with its code, message and cause. */
    constructor(code: PackageErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "PackageError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}

/** Why a host cannot grant a sandbox ability: it has none of its kind, or cannot enforce it. */
export type SandboxErrorCode = "UNSUPPORTED" | "UNENFORCEABLE";

/** A workload's sandbox ability its host cannot grant, refused before the workload starts. */
export class SandboxError extends Error implements DomainError {
    /** Stable failure code. */
    readonly code: SandboxErrorCode;
    /** The ability the host cannot grant. */
    readonly ability: SandboxName;

    /** Create a sandbox error with its code, the ability and the gap. */
    constructor(
        code: SandboxErrorCode,
        ability: SandboxName,
        message: string,
        options?: ErrorOptions,
    ) {
        // keep the code and the ability beside the message
        super(message, options);
        this.name = "SandboxError";
        this.code = code;
        this.ability = ability;
    }

    /** Report an ability no host grants as a workload the host cannot process. */
    toCallError(): Failure {
        return { code: "UNPROCESSABLE_CONTENT", message: this.message };
    }
}
