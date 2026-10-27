import {
    defineExample,
    defineScenario,
    type Given,
    SetStep,
    type Interaction,
} from "@destack/package/declare";
import { SimulatedClock } from "@destack/environment/simulation";
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
    globals: schema.object({}),
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
export const createTallyTwoMarks = defineExample({
    of: createTally,
    name: "two-marks",
    description: "a tally starting at two marks",
    properties: { marks: 2 },
    render: (properties) => createTally({ marks: 0, ...properties }),
});

/** Mark a tally twice, then set it back. */
export const createTallyMarkTwice = defineScenario({
    of: createTally,
    interaction: tallyInteraction,
    name: "mark-twice",
    description: "mark a tally twice, then set it back",
    given: { examples: [createTallyTwoMarks] },
    when: [{ action: "mark" }, { action: "mark" }, { action: "set", properties: { marks: 0 } }],
    then: {
        observe: { marks: { kind: "marks" } },
        each: [{ marks: "3" }, { marks: "4" }, { marks: "0" }],
    },
});

/** The clock tallies run on, which never moves. */
const tallyClock = new SimulatedClock(0);

/** The drivers of tallies. */
const TallyDriver: DriverType<Tallying, Tally> = {
    interaction: tallyInteraction,
    start(given: Given<Tally>): Driver<Tallying> {
        const render = given.examples[0]?.render;
        if (render === undefined) {
            throw new TypeError("no tally is given");
        }
        const tally = render();

        return {
            clock: tallyClock,
            act(step) {
                const set = SetStep.safeParse(step);
                tally.marks = set.success ? Number(set.data.properties["marks"]) : tally.marks + 1;
            },
            observe: () => String(tally.marks),
            count: () => ({ marks: tally.marks }),
        };
    },
};

test("observe a scenario's named observations after each step, or once after the last", async () => {
    const atEnd = {
        ...createTallyMarkTwice,
        then: { observe: createTallyMarkTwice.then.observe, end: { marks: "0" } },
    };
    expect([
        (await Runner.measure(createTallyMarkTwice, TallyDriver)).map((play) => play.observed),
        (await Runner.measure(atEnd, TallyDriver)).map((play) => play.observed),
    ]).toEqual([[[{ marks: "3" }, { marks: "4" }, { marks: "0" }]], [[{ marks: "0" }]]]);
});

test("measure each step's counters per play and write them as Go benchmark lines", async () => {
    // play twice, as a repeated measurement does
    const played = await Runner.measure(createTallyMarkTwice, TallyDriver, 2);

    // expect each step's counter growth and world time, the host's CPU time left out as it varies
    const counted = played.map(({ costs }) =>
        costs.map(({ world, counters }) => ({ world, counters })),
    );
    const steps = [
        { world: 0, counters: { marks: 1 } },
        { world: 0, counters: { marks: 1 } },
        { world: 0, counters: { marks: -4 } },
    ];
    expect(counted).toEqual([steps, steps]);

    // expect one line per step and play, the CPU time written in place
    const lines = Runner.format(createTallyMarkTwice, played)
        .trimEnd()
        .split("\n")
        .map((line) => line.replace(/^(\S+ 1 )\d+ cpu-ns/u, "$1N cpu-ns"));
    const play = [
        "BenchmarkMarkTwice/1-mark 1 N cpu-ns/op 0 world-ms/op 1 marks/op",
        "BenchmarkMarkTwice/2-mark 1 N cpu-ns/op 0 world-ms/op 1 marks/op",
        "BenchmarkMarkTwice/3-set 1 N cpu-ns/op 0 world-ms/op -4 marks/op",
    ];
    expect(lines).toEqual([...play, ...play]);
});
