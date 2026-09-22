/** An invalid space configuration. */
export class SpaceError extends Error {
    /** Stable failure code. */
    readonly code: "INVALID_DEFINITION";

    /** Create a configuration failure. */
    constructor(code: "INVALID_DEFINITION", message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SpaceError";
        this.code = code;
    }
}
