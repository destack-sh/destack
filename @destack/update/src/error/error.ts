import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code of each update failure. */
const CALL_CODES = {
    UPDATE_RUNNING: "CONFLICT",
    SESSION_CLOSED: "SERVICE_UNAVAILABLE",
    INVALID_REPOSITORY: "BAD_REQUEST",
    CACHE_MISMATCH: "CONFLICT",
    RELEASE_NOT_FOUND: "NOT_FOUND",
    INVALID_RELEASE: "BAD_GATEWAY",
    RELEASE_CHANGED: "CONFLICT",
    INSTALLATION_CHANGED: "CONFLICT",
    VERIFICATION_FAILED: "BAD_GATEWAY",
    DOWNLOAD_FAILED: "BAD_GATEWAY",
    UNSAFE_ARCHIVE: "BAD_GATEWAY",
    DOWNGRADE_REFUSED: "CONFLICT",
    ALREADY_INSTALLED: "CONFLICT",
    PLATFORM_CHANGED: "CONFLICT",
    TARGET_MISMATCH: "CONFLICT",
    UNSUPPORTED_TARGET: "BAD_REQUEST",
    INVALID_INSTALLATION: "PRECONDITION_FAILED",
    DEVELOPMENT_BUILD: "PRECONDITION_FAILED",
    ACTIVATION_FAILED: "INTERNAL_SERVER_ERROR",
    INSTALL_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** The stable causes of update and installation failures. */
export type UpdateReason = keyof typeof CALL_CODES;

/** A failure reported by the distribution updater. */
export class UpdateError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: UpdateReason;

    /** Describe an update failure and retain its underlying cause. */
    constructor(reason: UpdateReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "UpdateError";
        this.reason = reason;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return {
            code: CALL_CODES[this.reason],
            reason: this.reason,
            domain: import.meta.destack.package.name,
            message: this.message,
        };
    }
}
