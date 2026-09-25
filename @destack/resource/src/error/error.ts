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

/** A desired state no plan can reach until its declarations change. */
export class PlanError extends Error {
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
}
