import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each sandbox failure: an unsupported restriction, or the host's. */
const SERVICE_CODES = {
    UNSUPPORTED: "NOT_IMPLEMENTED",
    START_FAILED: "INTERNAL_SERVER_ERROR",
    STOP_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failed sandbox operation. */
export type SandboxErrorCode = keyof typeof SERVICE_CODES;

/** A process could not start or stop under the requested OS restrictions. */
export class SandboxError extends Error implements DomainError {
    /** The failed sandbox operation. */
    readonly code: SandboxErrorCode;

    /** Retain the operation and its underlying failure. */
    constructor(code: SandboxErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SandboxError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
