/** An invalid account declaration. */
export class AccountError extends Error {
    /** The stable failure code. */
    readonly code: "INVALID_DEFINITION";

    /** Create an account declaration failure. */
    constructor(code: "INVALID_DEFINITION", message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "AccountError";
        this.code = code;
    }
}
