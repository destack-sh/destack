import type { ReportableError, ServiceErrorCode, ServiceErrorReport } from "@destack/schema";

/** The service error code of each key failure: every key failure is the host's. */
const SERVICE_CODES = {
    KEY_UNAVAILABLE: "INTERNAL_SERVER_ERROR",
    DECRYPTION_FAILED: "INTERNAL_SERVER_ERROR",
    INVALID_KEY: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of a host's keys. */
export type HostErrorCode = keyof typeof SERVICE_CODES;

/** A failure of a host's keys, without their bytes. */
export class HostError extends Error implements ReportableError {
    /** The stable failure code. */
    readonly code: HostErrorCode;

    /** Report a failure without its cryptographic inputs. */
    constructor(code: HostErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "HostError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
