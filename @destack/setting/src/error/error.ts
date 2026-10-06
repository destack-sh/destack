import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each setting failure. */
const SERVICE_CODES = {
    UNDECLARED: "NOT_FOUND",
    INVALID_VALUE: "BAD_REQUEST",
    INVALID_PLACEMENT: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of setting declarations, placements and resolution. */
export type SettingErrorCode = keyof typeof SERVICE_CODES;

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

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
