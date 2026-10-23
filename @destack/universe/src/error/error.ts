import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code of each universe failure. */
const CALL_CODES = {
    DEFINITION_NOT_APPLIED: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of universe failures. */
export type UniverseReason = keyof typeof CALL_CODES;

/** A universe failure with its reason. */
export class UniverseError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: UniverseReason;

    /** Create a universe failure. */
    constructor(reason: UniverseReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "UniverseError";
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
