import {
    compareJsonSchemas,
    defineSchema,
    schema,
    type JsonSchema,
    type Version,
} from "@destack/schema";
import { digest } from "@destack/schema/json";
import { PlanError } from "../error/error.ts";

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

/** Which readers must accept a changed schema: newer readers of older values, or older readers of newer values. */
export type Compatibility = "backward" | "forward";

/** Compare two releases' descriptions of one declaration, throwing a PlanError for changes the later release must declare. */
export type Compare<Description = unknown> = (before: Description, after: Description) => Plan;

/** A schema change between two releases of a declaration. */
export interface SchemaChange {
    /** The changed part, such as a setting's value. */
    readonly target: string;
    /** The earlier release's schema. */
    readonly before: JsonSchema;
    /** The later release's schema. */
    readonly after: JsonSchema;
    /** The later release. */
    readonly release: Version;
    /** Which readers must accept the change. */
    readonly compatibility: Compatibility;
    /** Whether the later release declares a conversion from earlier values. */
    readonly isConverted: boolean;
}

/** The changes taking a resource from its applied state to its desired state. */
export const Plan = { classify, digest: digestSteps, schema: planSchema };

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

/** Plan a schema change: widening for backward readers, narrowing for forward ones, converting otherwise. */
function planSchema(change: SchemaChange): Plan {
    // compare the accepted values and keep an unchanged schema out of the plan
    const { target, release, compatibility } = change;
    const compared = compareJsonSchemas(change.before, change.after);
    const isCompatible =
        compared === "same" || compared === (compatibility === "backward" ? "wider" : "narrower");
    if (compared === "same") {
        return { steps: [] };
    }

    // accept a compatible change as safe
    if (isCompatible) {
        return {
            steps: [{ kind: compared, risk: "safe", target, detail: `${compared} ${target}` }],
        };
    }
    // convert earlier values for newer readers
    else if (compatibility === "backward" && change.isConverted) {
        const detail = `convert ${target} to ${release}`;

        return { steps: [{ kind: "convert", risk: "data-dependent", target, detail }] };
    }
    // require a conversion the release lacks
    else if (compatibility === "backward") {
        throw new PlanError([{ target, detail: `declare a conversion for ${release}` }]);
    }
    // keep earlier releases serving their readers
    else {
        const detail = `${compared} ${target}: earlier readers keep their release`;

        return { steps: [{ kind: compared, risk: "backward-incompatible", target, detail }] };
    }
}
