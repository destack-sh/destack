import { defineSchema, schema } from "@destack/schema";
import { digest } from "@destack/schema/json";

/** How consequential a step is, from least to most. */
export const Risk = defineSchema(
    schema.enum(["safe", "data-dependent", "backward-incompatible", "destructive"]),
);
/** How consequential a step is, from least to most. */
export type Risk = schema.Infer<typeof Risk>;

/** One change a plan makes to a resource, as reviewers see it. */
export interface Step {
    /** What the step changes, such as addColumn. */
    readonly kind: string;
    /** How consequential the step is. */
    readonly risk: Risk;
    /** The changed part of the resource, such as a table. */
    readonly target: string;
    /** A readable summary, such as "add column priority". */
    readonly detail: string;
}

/** The changes taking a resource from its applied state to its desired state. */
export interface Plan<Change extends Step = Step> {
    /** The steps in application order. */
    readonly steps: readonly Change[];
}

/** The changes taking a resource from its applied state to its desired state. */
export const Plan = { classify, digest: digestSteps };

/** Classify a plan by its most consequential step, safe when empty. */
function classify(plan: Plan): Risk {
    return plan.steps.reduce<Risk>(
        (highest, next) =>
            Risk.options.indexOf(next.risk) > Risk.options.indexOf(highest) ? next.risk : highest,
        "safe",
    );
}

/** Digest a plan's reviewed steps, so an apply can require the plan a review saw. */
function digestSteps(plan: Plan): Promise<string> {
    return digest(
        plan.steps.map((step) => ({
            kind: step.kind,
            risk: step.risk,
            target: step.target,
            detail: step.detail,
        })),
    );
}
