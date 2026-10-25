import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** What a resource's declarations must change, by the part of the resource it concerns. */
export const PlanProblem = schema.object({
    /** The part of the resource, such as a table. */
    target: schema.string(),
    /** What to declare, such as "declare a conversion to version 2". */
    detail: schema.string(),
});
/** What a resource's declarations must change. */
export type PlanProblem = schema.Infer<typeof PlanProblem>;

/** The problems of a desired state no plan reaches until its declarations change. */
export const UnreachablePlan = schema.object({
    /** Every problem at once. */
    problems: schema.array(PlanProblem),
});
/** The problems of a desired state no plan reaches. */
export type UnreachablePlan = schema.Infer<typeof UnreachablePlan>;

/** The resource package's failures, each reason with the call error code it answers. */
export const ResourceError = defineErrorFamily(
    import.meta.destack.package,
    {
        // conflict
        PLAN_UNREACHABLE: { code: "CONFLICT", data: UnreachablePlan },

        // internal server error
        ALREADY_BOUND: { code: "INTERNAL_SERVER_ERROR" },
        BINDING_MISSING: { code: "INTERNAL_SERVER_ERROR" },
        RESOURCE_UNPROVISIONED: { code: "INTERNAL_SERVER_ERROR" },
    },
    { audience: "host" },
);

/** A stable cause of resource failures. */
export type ResourceReason = ReasonOf<typeof ResourceError>;

/** Refuse a desired state no plan reaches until its declarations change, listing every problem. */
export function unreachablePlan(problems: readonly PlanProblem[]): FamilyError<"PLAN_UNREACHABLE"> {
    const message = problems.map((problem) => `${problem.target}: ${problem.detail}`).join("; ");

    return ResourceError.of("PLAN_UNREACHABLE", message, { data: { problems: [...problems] } });
}
