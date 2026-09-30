/** A failure code for space configurations and their administration. */
export type SpaceErrorCode =
    | "INVALID_DEFINITION"
    | "UNSUPPORTED_DEFINITION"
    | "NOT_READY"
    | "NOT_IMPLEMENTED"
    | "FENCED";

/** An invalid or unsupported space configuration, or a write its cell may no longer make. */
export class SpaceError extends Error {
    /** Stable failure code. */
    readonly code: SpaceErrorCode;

    /** Create a configuration failure. */
    constructor(code: SpaceErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SpaceError";
        this.code = code;
    }
}
