import type { Hue } from "@destack/color";
import type { IconBodies } from "@destack/icon";
import robotIcon from "@destack/icon/phosphor/robot";
import sparkleIcon from "@destack/icon/phosphor/sparkle";
import wrenchIcon from "@destack/icon/phosphor/wrench";
import type { TimelineItem } from "../timeline/index.ts";

/** What a step of the examples' run is. */
export type StepKind = "agent" | "model" | "tool";

/** A value a step took or gave, as JSON has it. */
export type StepValue =
    | string
    | number
    | boolean
    | null
    | readonly StepValue[]
    | { readonly [key: string]: StepValue };

/** A step of the examples' run. */
export interface Step {
    /** Its key. */
    readonly key: string;
    /** Its name, which the steps of one tool share. */
    readonly name: string;
    /** What it is. */
    readonly kind: StepKind;
    /** Its start, in seconds from the run's. */
    readonly start: number;
    /** Its end, in seconds from the run's start. */
    readonly end: number;
    /** The key of the step it is under. */
    readonly parent?: string;
    /** The tokens it took, when it asked a model. */
    readonly tokens?: number;
    /** Why it failed, when it did. */
    readonly error?: string;
    /** What it took. */
    readonly input?: StepValue;
    /** What it gave. */
    readonly output?: StepValue;
}

/** The start of the run, an instant in launch week. */
const START = Date.UTC(2026, 10, 19, 9, 0, 0);

/** The icon and color of each kind of step. */
export const KINDS: Readonly<
    Record<StepKind, { readonly icon: IconBodies; readonly label: string; readonly color: Hue }>
> = {
    agent: { icon: robotIcon, label: "Agent", color: "gray" },
    model: { icon: sparkleIcon, label: "Model", color: "purple" },
    tool: { icon: wrenchIcon, label: "Tool", color: "cyan" },
};

/** An agent's run: what it asked, the tools it used, and an agent it asked in turn. */
export const RUN: readonly Step[] = [
    {
        key: "run",
        name: "Compare the quotes",
        kind: "agent",
        start: 0,
        end: 14.2,
        input: "Which of the three quotes is cheapest in euros?",
        output: "The second one, at €3,900 a year.",
    },
    {
        key: "plan",
        name: "Plan the steps",
        kind: "model",
        start: 0.1,
        end: 1.9,
        parent: "run",
        tokens: 1_204,
        output: ["read the quotes", "read the terms", "compare in euros"],
    },
    {
        key: "quotes",
        name: "read_file",
        kind: "tool",
        start: 2,
        end: 3.1,
        parent: "run",
        input: { path: "quotes.pdf" },
        output: {
            pages: 3,
            prices: [
                { amount: 4_200, currency: "EUR" },
                { amount: 3_900, currency: "EUR" },
                { amount: 4_050, currency: "CHF" },
            ],
        },
    },
    { key: "terms", name: "Read the terms", kind: "agent", start: 3.2, end: 10.4, parent: "run" },
    {
        key: "open",
        name: "read_file",
        kind: "tool",
        start: 3.3,
        end: 4,
        parent: "terms",
        input: { path: "terms.pdf" },
        output: { pages: 12 },
    },
    {
        key: "rates-1",
        name: "fetch_rates",
        kind: "tool",
        start: 4.1,
        end: 4.8,
        parent: "terms",
        error: "503 Service Unavailable",
        input: { base: "EUR", currencies: ["CHF"] },
        output: { status: 503, message: "Service Unavailable" },
    },
    {
        key: "rates-2",
        name: "fetch_rates",
        kind: "tool",
        start: 5,
        end: 5.7,
        parent: "terms",
        error: "503 Service Unavailable",
        input: { base: "EUR", currencies: ["CHF"] },
        output: { status: 503, message: "Service Unavailable" },
    },
    {
        key: "rates-3",
        name: "fetch_rates",
        kind: "tool",
        start: 5.9,
        end: 6.3,
        parent: "terms",
        input: { base: "EUR", currencies: ["CHF"] },
        output: { CHF: 0.94 },
    },
    {
        key: "extract",
        name: "Extract the prices",
        kind: "model",
        start: 6.5,
        end: 10.2,
        parent: "terms",
        tokens: 3_980,
    },
    {
        key: "answer",
        name: "Write the summary",
        kind: "model",
        start: 10.5,
        end: 14.1,
        parent: "run",
        tokens: 2_417,
        output: "The second one, at €3,900 a year.",
    },
];

/** The verbs of steps. */
export const Step = {
    /** Read a step of the run, refusing a key it does not have. */
    of(key: string): Step {
        const step = RUN.find((each) => each.key === key);
        if (step === undefined) {
            throw new TypeError(`step without an entry: '${key}'`);
        }

        return step;
    },

    /** Read the color of a step: its kind's, red once it failed. */
    hue(step: Step): Hue {
        return step.error === undefined ? KINDS[step.kind].color : "red";
    },

    /** Report whether a step or one under it failed. */
    hasFailed(step: Step): boolean {
        return (
            step.error !== undefined ||
            RUN.some((each) => each.parent === step.key && Step.hasFailed(each))
        );
    },

    /** Draw a step on a timeline. */
    item(step: Step): TimelineItem {
        return {
            key: step.key,
            label: step.name,
            start: START + step.start * 1_000,
            end: START + step.end * 1_000,
            ...(step.parent === undefined ? {} : { parent: step.parent }),
            color: Step.hue(step),
            variant: "line",
        };
    },
};
