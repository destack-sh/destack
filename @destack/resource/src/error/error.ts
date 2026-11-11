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
export const ResourceError = defineErrorFamily({
    reasons: {
        // not found
        BLOB_NOT_FOUND: {},

        // invalid
        DIGEST_INVALID: {},

        // required
        TEXT_REQUIRED: {},

        // blocked
        PLAN_BLOCKED: { data: UnreachablePlan },
        PROVIDER_BLOCKED: {},

        // unsupported
        PROVISIONING_UNSUPPORTED: {},

        // unavailable
        RESOURCE_UNAVAILABLE: {},

        // broken
        BINDING_BROKEN: {},
        CREDENTIAL_BROKEN: {},
    },
    audience: "host",
});

/** A stable cause of resource failures. */
export type ResourceReason = ReasonOf<typeof ResourceError>;

/** Refuse a desired state no plan reaches until its declarations change, listing every problem. */
export function unreachablePlan(problems: readonly PlanProblem[]): FamilyError<"PLAN_BLOCKED"> {
    const detail = problems.map((problem) => `${problem.target}: ${problem.detail}`).join("; ");

    return ResourceError.of("PLAN_BLOCKED", `cannot reach the desired state: ${detail}`, {
        data: { problems: [...problems] },
    });
}
