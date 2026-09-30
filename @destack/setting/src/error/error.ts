import { ServiceError } from "@destack/service/error";

/** The service error code of each setting failure. */
const SERVICE_CODES = {
    UNDECLARED: "NOT_FOUND",
    INVALID_VALUE: "BAD_REQUEST",
    INVALID_PLACEMENT: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
} as const;

/** A setting declaration, placement or resolution failure. */
export class SettingError extends Error {
    /** The failure category. */
    readonly code: keyof typeof SERVICE_CODES;

    /** Report a setting failure. */
    constructor(code: SettingError["code"], message: string) {
        super(message);
        this.name = "SettingError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError() {
        return new ServiceError(SERVICE_CODES[this.code], { message: this.message, cause: this });
    }
}
