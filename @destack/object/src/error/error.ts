/** Stable object failure codes. */
export type ObjectErrorCode =
    | "INVALID_DECLARATION"
    | "UNSUPPORTED_DECLARATION"
    | "CYCLIC_DECLARATION";

/** An object failure with a stable code. */
export class ObjectError extends Error {
    /** The failure code. */
    readonly code: ObjectErrorCode;

    /** Create an object failure. */
    constructor(code: ObjectErrorCode, message: string) {
        super(message);
        this.name = "ObjectError";
        this.code = code;
    }
}
