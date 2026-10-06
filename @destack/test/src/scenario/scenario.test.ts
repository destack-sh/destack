import {
    defineExample,
    defineScenario,
    type Given,
    SetStep,
    type Interaction,
} from "@destack/package/declare";
import { schema } from "@destack/schema";
import { expect, test } from "vitest";
import { type Driver, type DriverType, Runner } from "./scenario.ts";

/** A tally's step: marking it once. */
const MarkStep = schema.object({ action: schema.literal("mark") });

/** A tally's observation: its marks. */
const MarksObservation = schema.object({ kind: schema.literal("marks") });

/** The interaction of tallies. */
type Tallying = Interaction<
    schema.Infer<typeof MarkStep>,
    schema.Infer<typeof MarksObservation>,
    Record<string, never>
>;

/** The interaction of tallies. */
const tallyInteraction: Tallying = {
    name: "tally",
    step: MarkStep,
    observation: MarksObservation,
    environment: schema.object({}),
};

/** A tally of marks. */
interface Tally {
    /** The marks so far. */
    marks: number;
}

/** Start a tally. */
function createTally(properties: { readonly marks: number }): Tally {
    return { marks: properties.marks };
}

/** A tally starting at two marks. */
const TwoMarks = defineExample({
    of: createTally,
    name: "two-marks",
    properties: { marks: 2 },
    render: (properties) => createTally({ marks: 0, ...properties }),
});

/** Mark a tally twice, then set it back. */
const MarkTwice = defineScenario({
    interaction: tallyInteraction,
    name: "mark a tally twice, then set it back",
    given: { examples: [TwoMarks] },
    when: [{ action: "mark" }, { action: "mark" }, { action: "set", properties: { marks: 0 } }],
    then: {
        observe: { marks: { kind: "marks" } },
        each: [{ marks: "3" }, { marks: "4" }, { marks: "0" }],
    },
});

/** The drivers of tallies: a mark step marks the tally, a set step replaces its marks, and its marks read as text. */
const TallyDriver: DriverType<Tallying, Tally> = {
    interaction: tallyInteraction,
    start(given: Given<Tally>): Driver<Tallying> {
        const render = given.examples[0]?.render;
        if (render === undefined) {
            throw new TypeError("no tally is given");
        }
        const tally = render();

        return {
            act(step) {
                const set = SetStep.safeParse(step);
                tally.marks = set.success ? Number(set.data.properties["marks"]) : tally.marks + 1;
            },
            observe: () => String(tally.marks),
        };
    },
};

test("observe a scenario's named observations after each step, or once after the last", async () => {
    const atEnd = { ...MarkTwice, then: { observe: MarkTwice.then.observe, end: { marks: "0" } } };
    expect([
        await Runner.observe(MarkTwice, TallyDriver),
        await Runner.observe(atEnd, TallyDriver),
    ]).toEqual([[{ marks: "3" }, { marks: "4" }, { marks: "0" }], [{ marks: "0" }]]);
});
