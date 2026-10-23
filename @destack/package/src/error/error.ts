import { schema } from "@destack/schema";
import { type DomainError, type CallErrorCode, type Failure } from "@destack/error";
import type { SandboxName } from "../definition/sandbox.ts";

/** The domain of package failures, written out: the Bun preload loads them before stamping. */
const DOMAIN = "@destack/package";

/** Invalid package definitions and distributed files. */
export const PackageReason = schema.enum([
    "INVALID_DEFINITION",
    "UNSUPPORTED_LANGUAGE",
    "UNSUPPORTED_TARGET",
    "INVALID_EXPORT",
    "INVALID_DEPENDENCY",
    "INVALID_FILE",
    "INVALID_INSPECTION",
    "INVALID_BUILD",
]);

/** A package failure's stable cause. */
export type PackageReason = schema.Infer<typeof PackageReason>;

/** The call error code of each package failure: an invalid package, or one no host processes. */
const CALL_CODES: Readonly<Record<PackageReason, CallErrorCode>> = {
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
    /** The failure's stable cause within its package. */
    readonly reason: PackageReason;

    /** Create a package error with its reason, message and cause. */
    constructor(reason: PackageReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "PackageError";
        this.reason = reason;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return {
            code: CALL_CODES[this.reason],
            reason: this.reason,
            domain: DOMAIN,
            message: this.message,
        };
    }
}

/** Why a host cannot grant a sandbox ability: it has none of its kind, or cannot enforce it. */
export type SandboxReason = "ABILITY_UNSUPPORTED" | "ABILITY_UNENFORCEABLE";

/** A workload's sandbox ability its host cannot grant, refused before the workload starts. */
export class SandboxError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: SandboxReason;
    /** The ability the host cannot grant. */
    readonly ability: SandboxName;

    /** Create a sandbox error with its reason, the ability and the gap. */
    constructor(
        reason: SandboxReason,
        ability: SandboxName,
        message: string,
        options?: ErrorOptions,
    ) {
        // keep the reason and the ability beside the message
        super(message, options);
        this.name = "SandboxError";
        this.reason = reason;
        this.ability = ability;
    }

    /** Report an ability no host grants as a workload the host cannot process. */
    toCallError(): Failure {
        return {
            code: "UNPROCESSABLE_CONTENT",
            reason: this.reason,
            domain: DOMAIN,
            message: this.message,
        };
    }
}
