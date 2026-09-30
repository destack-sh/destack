import {
    compareJsonSchemas,
    defineSchema,
    schema,
    type JsonSchema,
    Version,
} from "@destack/schema";
import { digest } from "@destack/schema/json";
import type { DeclarationDescription } from "@destack/package/inspect";
import { PlanError } from "../error/error.ts";
import { Address } from "./address.ts";

/** How consequential a step is, from least to most. */
export const Risk = defineSchema(
    schema.enum(["safe", "data-dependent", "backward-incompatible", "destructive"]),
);
/** How consequential a step is, from least to most. */
export type Risk = schema.Infer<typeof Risk>;

/** What a step does to its target, after Terraform's plan actions. */
export const Action = defineSchema(
    schema.enum(["create", "update", "delete", "replace", "rename", "convert", "restore"]),
);
/** What a step does to its target. */
export type Action = schema.Infer<typeof Action>;

/** One change a plan makes, as reviewers see it. */
export const Step = defineSchema(
    schema.object({
        /** What the step does. */
        action: Action,
        /** The changed part. */
        target: Address,
        /** How consequential the step is. */
        risk: Risk,
        /** A readable summary, such as "add column priority". */
        detail: schema.string().min(1),
        /** Each changed field's current and planned value. */
        fields: schema
            .record(schema.string(), schema.object({ before: schema.json(), after: schema.json() }))
            .optional(),
    }),
);
/** One change a plan makes, as reviewers see it. */
export type Step = schema.Infer<typeof Step>;

/** The changes one review covers, in application order. */
export interface Plan<Change extends Step = Step> {
    /** The steps in application order. */
    readonly steps: readonly Change[];
    /** Why the plan stops before later steps. */
    readonly deferred?: string;
}

/** The changes one review covers. */
export const Plan = Object.assign(
    defineSchema(
        schema.object({
            /** The steps in application order. */
            steps: schema.array(Step),
            /** Why the plan stops before later steps. */
            deferred: schema.string().min(1).optional(),
        }),
    ),
    { classify, reaches, join, digest: digestSteps, values: planValues },
);

/** Which readers must accept a changed schema: newer readers of older values, or older readers of newer values. */
export type Compatibility = "backward" | "forward";

/** Compare two releases' manifest entries of one declaration, throwing a PlanError for changes the later release must declare. */
export type Compare = (
    before: Pick<DeclarationDescription, "description" | "symbol">,
    after: Pick<DeclarationDescription, "description" | "symbol">,
) => Plan;

/** A schema change between two releases of a declaration. */
export interface SchemaChange {
    /** The changed part, such as a setting's value. */
    readonly target: Address;
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

/** Classify a plan by its most consequential step, safe when empty. */
function classify(plan: Plan): Risk {
    return plan.steps.reduce<Risk>(
        (highest, next) =>
            Risk.options.indexOf(next.risk) > Risk.options.indexOf(highest) ? next.risk : highest,
        "safe",
    );
}

/** Decide whether a plan reaches a risk. */
function reaches(plan: Plan, risk: Risk): boolean {
    return (
        plan.steps.length > 0 && Risk.options.indexOf(classify(plan)) >= Risk.options.indexOf(risk)
    );
}

/** Join plans in order, collecting every refusal into one PlanError. */
function join(parts: readonly (() => Plan)[]): Plan {
    // run each part, keeping its steps or its problems
    const steps: Step[] = [];
    const problems: PlanError["problems"][number][] = [];
    for (const part of parts) {
        try {
            steps.push(...part().steps);
        } catch (error) {
            if (!(error instanceof PlanError)) {
                throw error;
            }
            problems.push(...error.problems);
        }
    }
    if (problems.length > 0) {
        throw new PlanError(problems);
    }

    return { steps };
}

/** Digest a plan's reviewed steps, so an apply can require the plan a review saw. */
function digestSteps(plan: Plan): Promise<string> {
    return digest(
        plan.steps.map((step) => ({
            action: step.action,
            risk: step.risk,
            target: step.target,
            detail: step.detail,
            ...(step.fields === undefined ? {} : { fields: step.fields }),
        })),
    );
}

/** Plan a value schema change: widening for backward readers, narrowing for forward ones, converting otherwise. */
function planValues(change: SchemaChange): Plan {
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
            steps: [{ action: "update", target, risk: "safe", detail: `${compared} values` }],
        };
    }
    // convert earlier values for newer readers
    else if (compatibility === "backward" && change.isConverted) {
        const detail = `convert values to ${release}`;

        return { steps: [{ action: "convert", target, risk: "data-dependent", detail }] };
    }
    // require a conversion the release lacks
    else if (compatibility === "backward") {
        throw new PlanError([{ target, detail: `declare a conversion for ${release}` }]);
    }
    // keep earlier releases serving their readers
    else {
        const detail = `${compared} values: earlier readers keep their release`;

        return { steps: [{ action: "update", target, risk: "backward-incompatible", detail }] };
    }
}
