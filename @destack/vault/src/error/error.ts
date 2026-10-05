/** A failure code of vault storage or encryption. */
export type VaultErrorCode = "KEY_UNAVAILABLE" | "DECRYPTION_FAILED";

/** A vault storage or encryption failure without secret contents. */
export class VaultError extends Error {
    /** The stable failure code. */
    readonly code: VaultErrorCode;

    /** Report a failure without its cryptographic inputs. */
    constructor(code: VaultErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "VaultError";
        this.code = code;
    }
}
