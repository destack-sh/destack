import {
    defineBenchmark,
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

/** Mark a tally four times after two unmeasured marks. */
export const markTally = defineBenchmark({
    of: createTally,
    interaction: tallyInteraction,
    name: "tally",
    description: "a tally marked four times after two setup marks",
    parameters: { start: 2 },
    setup: [{ action: "mark" }, { action: "mark" }],
    operations: { "tally.mark": { action: "mark" } },
    iterations: 4,
    limits: { "tally.mark": { marks: 1 } },
});

/** The clock tallies run on, which never moves. */
const tallyClock = new SimulatedClock(0);

/** The drivers of tallies. */
const TallyDriver: DriverType<Tallying, Tally> = {
    interaction: tallyInteraction,
    start(given: Given<Tally>): Driver<Tallying> {
        // start the given tally, or an empty one for a benchmark
        const tally = given.examples[0]?.render?.() ?? createTally({ marks: 0 });

        return {
            clock: tallyClock,
            act(step) {
                const set = SetStep.safeParse(step);
                tally.marks = set.success ? Number(set.data.properties["marks"]) : tally.marks + 1;
            },
            observe: () => String(tally.marks),
            count: () => ({ marks: tally.marks }),
            fingerprint: () => tally.marks,
            [Symbol.asyncDispose]: async () => undefined,
        };
    },
};

test("observe a scenario's named observations after each step, or once after the last", async () => {
    const atEnd = {
        ...createTallyMarkTwice,
        then: { observe: { marks: { kind: "marks" } }, end: { marks: "0" } },
    } as const;

    expect([
        (await Runner.observe(createTallyMarkTwice, TallyDriver)).result,
        (await Runner.observe(atEnd, TallyDriver)).result,
    ]).toEqual([[{ marks: "3" }, { marks: "4" }, { marks: "0" }], [{ marks: "0" }]]);
});

test("measure each operation per iteration after an unmeasured setup, as Go benchmark lines", async () => {
    // measure marking after two setup marks
    const { result: measured } = await Runner.measure(markTally, TallyDriver);

    // expect the counter growth of the measured iterations only, the host CPU left out as it varies
    expect(
        measured.map(({ operation, iterations, world, counters }) => ({
            operation,
            iterations,
            world,
            counters,
        })),
    ).toEqual([{ operation: "tally.mark", iterations: 4, world: 0, counters: { marks: 4 } }]);

    // expect configuration lines, then one line per operation with values per iteration
    const lines = Runner.format(markTally, measured, { pkg: "@destack/test", database: "sqlite" })
        .replace(/ \S+ cpu-sec/u, " N cpu-sec")
        .split("\n");
    expect(lines).toEqual([
        "pkg: @destack/test",
        "database: sqlite",
        "",
        "BenchmarkTally/op=tally.mark/start=2 4 0 sec/op N cpu-sec/op 1 marks/op",
        "",
    ]);
});

test("refuse a replay whose second play diverges, naming the first step after which it did", async () => {
    // play a tally whose second play marks once more after its second step
    const plays = [
        [1, 2, 3],
        [1, 3, 4],
    ];
    const play = async () => {
        const fingerprints = plays.shift() ?? [];

        return {
            result: "played",
            steps: fingerprints.map((fingerprint, index) => ({
                step: `mark ${index}`,
                fingerprint,
            })),
        };
    };

    // expect the replay to refuse the second step
    await expect(Runner.replay(play)).rejects.toThrow(
        new Error("the world diverged between two plays of one seed at step 1: mark 1"),
    );
});
