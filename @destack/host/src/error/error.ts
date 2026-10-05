/** A failure code of a host's keys. */
export type HostErrorCode = "KEY_UNAVAILABLE" | "DECRYPTION_FAILED" | "INVALID_KEY";

/** A failure of a host's keys, without their bytes. */
export class HostError extends Error {
    /** The stable failure code. */
    readonly code: HostErrorCode;

    /** Report a failure without its cryptographic inputs. */
    constructor(code: HostErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "HostError";
        this.code = code;
    }
}
