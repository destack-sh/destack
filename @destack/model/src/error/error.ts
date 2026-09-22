/** An invalid standard model declaration. */
export class ModelError extends Error {
    /** The stable failure code. */
    readonly code: "INVALID_DEFINITION";

    /** Create a model declaration failure. */
    constructor(code: "INVALID_DEFINITION", message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "ModelError";
        this.code = code;
    }
}
