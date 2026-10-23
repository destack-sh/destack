import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code of each setting failure. */
const CALL_CODES = {
    UNDECLARED: "NOT_FOUND",
    UNKNOWN_SCOPE: "NOT_FOUND",
    INVALID_VALUE: "BAD_REQUEST",
    INVALID_PLACEMENT: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of failures of setting declarations, placements and resolution. */
export type SettingReason = keyof typeof CALL_CODES;

/** A setting declaration, placement or resolution failure. */
export class SettingError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: SettingReason;

    /** Report a setting failure. */
    constructor(reason: SettingReason, message: string) {
        super(message);
        this.name = "SettingError";
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
