import {
    defineExample,
    defineScenario,
    type Given,
    SetStep,
    type Interaction,
} from "@destack/package/declare";
import {
    SimulatedClock,
    SimulatedRandom,
    SimulatedTelemetry,
} from "@destack/environment/simulation";
import { schema } from "@destack/schema";
import { expect, test } from "vitest";
import { type Driver, type DriverType, Runner } from "./scenario.ts";

/** A tally's step: marking it once. */
const MarkStep = schema.object({ action: schema.literal("mark") });

/** A tally's observation: its marks. */
const MarksObservation = schema.object({ kind: schema.literal("marks") });

/** A tally's step. */
type Marking = schema.Infer<typeof MarkStep>;

/** A tally's observation. */
type Marks = schema.Infer<typeof MarksObservation>;

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
    parameters: {},
    when: [
        { step: { action: "mark" }, then: [{ observe: { kind: "marks" }, is: "3" }] },
        { step: { action: "mark" }, then: [{ observe: { kind: "marks" }, is: "4" }] },
        {
            step: { action: "set", properties: { marks: 0 } },
            then: [{ observe: { kind: "marks" }, is: "0" }],
        },
    ],
});

/** Mark a tally twice, then measure marking it four times after one warm-up mark. */
export const markTally = defineScenario({
    of: createTally,
    interaction: tallyInteraction,
    name: "tally",
    description: "mark a tally twice, then measure marking it four times after one warm-up mark",
    given: { examples: [createTallyTwoMarks] },
    parameters: { marks: 4 },
    when: [
        { action: "mark" },
        { action: "mark" },
        {
            operation: "tally.mark",
            times: "{marks}",
            warmup: 1,
            step: { action: "mark" },
            then: [
                { spans: { name: "tally.mark", host: "tally" }, count: "{marks}" },
                { logs: { severity: "ERROR" }, count: 0 },
                { metric: { name: "tally.marks" }, atMost: 1 },
            ],
        },
    ],
    then: [{ observe: { kind: "marks" }, is: "9" }],
});

/** The clock tallies run on, which never moves. */
const tallyClock = new SimulatedClock(0);

/** The host a tally's telemetry names. */
const TALLY_HOST = "tally";

/** The drivers of tallies. */
const TallyDriver: DriverType<Marking, Marks, Tally> = {
    interaction: tallyInteraction,
    configuration: {},
    start(given: Given<Tally>): Driver<Marking, Marks> {
        // start the given tally, recording a span and a counted mark per mark on its host
        const tally = given.examples[0]?.render?.() ?? createTally({ marks: 0 });
        const telemetry = new SimulatedTelemetry({
            clock: tallyClock,
            random: new SimulatedRandom(1),
        });
        const host = telemetry.host(TALLY_HOST);
        const marks = host.meters.getMeter("tally").createCounter("tally.marks");

        return {
            clock: tallyClock,
            telemetry,
            act(step) {
                const set = SetStep.safeParse(step);
                if (set.success) {
                    tally.marks = Number(set.data.properties["marks"]);
                    if (tally.marks < 0) {
                        host.loggers
                            .getLogger("tally")
                            .emit({ severityText: "ERROR", body: "negative marks" });
                    }
                } else {
                    host.tracers.getTracer("tally").startSpan("tally.mark").end();
                    marks.add(1);
                    tally.marks += 1;
                }
            },
            observe: () => String(tally.marks),
            fingerprint: () => tally.marks,
            [Symbol.asyncDispose]: async () => undefined,
        };
    },
};

test("check observations after steps and at the end", async () => {
    // expect other marks after the second step and at the end
    const misread = {
        ...createTallyMarkTwice,
        when: [
            { action: "mark" },
            { step: { action: "mark" }, then: [{ observe: { kind: "marks" }, is: "5" }] },
        ],
        then: [{ observe: { kind: "marks" }, is: "0" }],
    } as const;

    expect([
        (await Runner.playthrough(createTallyMarkTwice, TallyDriver)).failures,
        (await Runner.playthrough(misread, TallyDriver)).failures,
    ]).toEqual([
        [],
        [
            '{"action":"mark"} observed {"kind":"marks"} as "4", not "5"',
            'the end observed {"kind":"marks"} as "4", not "0"',
        ],
    ]);
});

test("measure an operation in Go benchmark lines", async () => {
    // mark four times after two unmeasured marks and one warm-up mark
    const {
        measurements: measured,
        failures,
        fingerprints,
        errors,
    } = await Runner.playthrough(markTally, TallyDriver);
    expect([failures, fingerprints, errors]).toEqual([[], [3, 4, 9], []]);

    // expect the metric growth of the measured repetitions only and no failed check, the host CPU left out as it varies
    expect(
        measured.map(({ operation, times, world, counters, failures: failed }) => ({
            operation,
            times,
            world,
            counters,
            failures: failed,
        })),
    ).toEqual([
        {
            operation: "tally.mark",
            times: 4,
            world: 0,
            counters: { "tally.marks": 4, "tally.marks@tally": 4 },
            failures: [],
        },
    ]);

    // expect configuration lines, then one line per operation with values per repetition, host times left out as they vary
    const lines = Runner.format(markTally, measured, { pkg: "@destack/test", database: "sqlite" })
        .replaceAll(/ \S+ (sec|p50-sec|p95-sec|cpu-sec)\/op/gu, " N $1/op")
        .split("\n");
    expect(lines).toEqual([
        "pkg: @destack/test",
        "database: sqlite",
        "",
        "BenchmarkTally/marks=4/op=tally.mark 4 N sec/op N p50-sec/op N p95-sec/op N cpu-sec/op 0 world-sec/op 1 tally.marks/op 1 tally.marks@tally/op",
        "",
    ]);
});

test("name the first step two plays differ after", () => {
    // compare a play with itself and with one that marks once more after its second step
    const play = { failures: [], measurements: [], fingerprints: [3, 4, 8], errors: [] };
    const diverging = { ...play, fingerprints: [3, 5, 9] };

    expect([
        Runner.checkDeterminism(markTally, play, play),
        Runner.checkDeterminism(markTally, play, diverging),
    ]).toEqual([undefined, 'step 1: {"action":"mark"}']);
});

test("fail checks the telemetry breaks", async () => {
    // expect other counts of the marks' spans, records and growth, and bounds a mark breaks
    const overchecked = {
        ...markTally,
        when: [
            {
                operation: "tally.mark",
                times: 2,
                step: { action: "mark" },
                then: [
                    { spans: { name: "tally.mark", host: "t*y" }, count: 3 },
                    { spans: { name: "tally.mark" }, atMost: 0.5 },
                    { logs: { severity: "ERROR" }, count: 1 },
                    { metric: { name: "tally.marks", host: "tally" }, atMost: 0.5 },
                    { metric: { name: "tally.unmarks" }, atMost: 1 },
                    { metric: { name: "tally.marks" }, count: 3 },
                ],
            },
        ],
    } as const;
    const { measurements: measured } = await Runner.playthrough(overchecked, TallyDriver);

    expect(measured.flatMap(({ failures }) => failures)).toEqual([
        'tally.mark ended 2 spans {"name":"tally.mark","host":"t*y"}, not 3',
        'tally.mark ended 1 spans {"name":"tally.mark"} each, more than 0.5',
        'tally.mark logged 0 records {"severity":"ERROR"}, not 1',
        'tally.mark grew {"name":"tally.marks","host":"tally"} by 1, more than 0.5: 1 tally',
        'tally.mark grew no {"name":"tally.unmarks"}: bound a cost that occurs',
        'tally.mark grew {"name":"tally.marks"} by 2, not 3',
    ]);
});

test("collect a play's undeclared error records", async () => {
    // set a tally below zero, which logs an error
    const negative = {
        ...createTallyMarkTwice,
        when: [{ action: "set", properties: { marks: -1 } }],
        then: [{ observe: { kind: "marks" }, is: "-1" }],
    } as const;

    // expect the error as undeclared, then as declared by a check selecting it
    const declared = {
        ...negative,
        when: [
            {
                step: { action: "set", properties: { marks: -1 } },
                then: [{ logs: { severity: "ERROR", host: "tally" }, count: 1 }],
            },
        ],
    } as const;
    expect([
        (await Runner.playthrough(negative, TallyDriver)).errors,
        (await Runner.playthrough(declared, TallyDriver)).errors,
    ]).toEqual([["negative marks"], []]);
});
