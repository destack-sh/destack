import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code of each binding failure: every binding failure is the host's. */
const CALL_CODES = {
    NOT_BOUND: "INTERNAL_SERVER_ERROR",
    ALREADY_BOUND: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of resource bindings. */
export type ResourceErrorCode = keyof typeof CALL_CODES;

/** A resource binding failure. */
export class ResourceError extends Error implements DomainError {
    /** The binding operation that failed. */
    readonly code: ResourceErrorCode;

    /** Describe a missing or duplicate binding. */
    constructor(code: ResourceErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "ResourceError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}

/** A desired state no plan can reach until its declarations change. */
export class PlanError extends Error implements DomainError {
    /** What the declarations must change, by the part of the resource it concerns. */
    readonly problems: readonly {
        /** The part of the resource, such as a table. */
        readonly target: string;
        /** What to declare, such as "declare a conversion to version 2". */
        readonly detail: string;
    }[];

    /** Describe every problem at once. */
    constructor(problems: PlanError["problems"]) {
        super(problems.map((problem) => `${problem.target}: ${problem.detail}`).join("; "));
        this.name = "PlanError";
        this.problems = problems;
    }

    /** Report the unreachable state as a conflict with the declared state. */
    toCallError(): CallErrorReport {
        return { code: "CONFLICT", message: this.message };
    }
}
