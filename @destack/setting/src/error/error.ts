import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code of each setting failure. */
const CALL_CODES = {
    UNDECLARED: "NOT_FOUND",
    INVALID_VALUE: "BAD_REQUEST",
    INVALID_PLACEMENT: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of setting declarations, placements and resolution. */
export type SettingErrorCode = keyof typeof CALL_CODES;

/** A setting declaration, placement or resolution failure. */
export class SettingError extends Error implements DomainError {
    /** The failure category. */
    readonly code: SettingErrorCode;

    /** Report a setting failure. */
    constructor(code: SettingErrorCode, message: string) {
        super(message);
        this.name = "SettingError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
