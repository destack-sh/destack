import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each object failure: every declaration failure is the host's. */
const SERVICE_CODES = {
    INVALID_DECLARATION: "INTERNAL_SERVER_ERROR",
    UNSUPPORTED_DECLARATION: "INTERNAL_SERVER_ERROR",
    CYCLIC_DECLARATION: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** Stable object failure codes. */
export type ObjectErrorCode = keyof typeof SERVICE_CODES;

/** An object failure with a stable code. */
export class ObjectError extends Error implements DomainError {
    /** The failure code. */
    readonly code: ObjectErrorCode;

    /** Create an object failure. */
    constructor(code: ObjectErrorCode, message: string) {
        super(message);
        this.name = "ObjectError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
