/** A vault storage or encryption failure without secret contents. */
export class VaultError extends Error {
    /** Machine-readable failure category. */
    readonly code: "KEY_UNAVAILABLE" | "DECRYPTION_FAILED" | "INVALID_KEY";

    /** Report a safe failure without propagating cryptographic inputs. */
    constructor(code: VaultError["code"], message: string) {
        super(message);
        this.name = "VaultError";
        this.code = code;
    }
}
