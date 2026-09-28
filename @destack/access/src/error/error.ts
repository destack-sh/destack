import type { StepUp } from "../context/elevation.ts";

/** Reject an invalid declaration, unavailable record, or unauthorized operation. */
export class AccessError extends Error {
    /** The stable failure classification. */
    readonly code:
        | "INVALID_DECLARATION"
        | "INVALID_CONTEXT"
        | "NOT_FOUND"
        | "FORBIDDEN"
        | "INSUFFICIENT_AUTHENTICATION"
        | "CONFLICT"
        | "STALE";
    /** The authentication that would admit the caller, for insufficient authentication. */
    readonly stepUp?: StepUp;

    /** Retain the error classification and cause, and the authentication that would admit the caller. */
    constructor(
        code: AccessError["code"],
        message: string,
        options?: ErrorOptions & { readonly stepUp?: StepUp },
    ) {
        // classify the error
        super(message, options);
        this.name = "AccessError";
        this.code = code;

        // keep the challenge a refusal names
        if (options?.stepUp !== undefined) {
            this.stepUp = options.stepUp;
        }
    }
}
