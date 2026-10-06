import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each binding failure: every binding failure is the host's. */
const SERVICE_CODES = {
    NOT_BOUND: "INTERNAL_SERVER_ERROR",
    ALREADY_BOUND: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of resource bindings. */
export type ResourceErrorCode = keyof typeof SERVICE_CODES;

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

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
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
    toServiceError(): ServiceErrorReport {
        return { code: "CONFLICT", message: this.message };
    }
}
