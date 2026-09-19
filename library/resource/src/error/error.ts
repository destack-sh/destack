/** A resource binding failure. */
export class ResourceError extends Error {
    /** The binding operation that failed. */
    readonly code: "NOT_BOUND" | "ALREADY_BOUND";

    /** Describe a missing or duplicate binding. */
    constructor(code: ResourceError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "ResourceError";
        this.code = code;
    }
}
