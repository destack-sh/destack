import type { Given, Scenario, SetStep, Interaction } from "@destack/package/declare";
import type { Clock } from "@destack/environment";
import type { JsonValue, schema } from "@destack/schema";
import { join } from "node:path";
import { expect, test } from "vitest";
import { testHost } from "../host/index.ts";

/** The variable setting how often a play repeats to measure a scenario, once by default. */
const PLAYS_VARIABLE = "DESTACK_BENCH_PLAYS";

/** The variable naming the baseline a run's measurements are saved as, `current` by default. */
const BASELINE_VARIABLE = "DESTACK_BENCH_BASELINE";

/** The baseline a run's measurements are saved as without one named. */
const CURRENT_BASELINE = "current";

/** The directory of the saved measurements, below the package's working directory. */
const BENCH_DIRECTORY = "node_modules/.cache/destack/bench";

/** Nanoseconds per second, as process usage reads seconds and Go's format writes nanoseconds. */
const NANOSECONDS = 1e9;

/** A driver that plays one scenario's steps and reads its observations, after WebDriver. */
export interface Driver<Speaks extends Interaction = Interaction> {
    /** The clock the driver's world runs on, which measures each step's world time. */
    readonly clock: Clock;
    /** Take one step. */
    act(step: SetStep | schema.Infer<Speaks["step"]>): void | Promise<void>;
    /** Read one observation. */
    observe(observation: schema.Infer<Speaks["observation"]>): JsonValue | Promise<JsonValue>;
    /** Read the world's counters since the driver started, such as statements, by unit. */
    readonly count: () => Counters | Promise<Counters>;
}

/** The drivers of one interaction, each started on the examples a scenario is given. */
export interface DriverType<Speaks extends Interaction = Interaction, Instance = unknown> {
    /** The interaction the drivers speak. */
    readonly interaction: Speaks;
    /** Start a driver on the examples a scenario is given, in its environment. */
    start(given: Given<Instance>): Driver<Speaks> | Promise<Driver<Speaks>>;
}

/** The values of a scenario's named observations at one point. */
export type Observations = Record<string, JsonValue>;

/** A world's counters by unit, such as `statements`, cumulative since its driver started. */
export type Counters = Readonly<Record<string, number>>;

/** One step's cost in one play: host CPU, world time and each counter's growth. */
export interface StepCost {
    /** The host CPU time the step took, in nanoseconds. */
    readonly cpu: number;
    /** The world time the step took on the driver's clock, in milliseconds. */
    readonly world: number;
    /** Each counter's growth over the step, by unit. */
    readonly counters: Counters;
}

/** One play of a scenario: what it observed and what each step cost. */
export interface Play {
    /** The observations after each step, or after the last. */
    readonly observed: Observations[];
    /** The step costs, in step order. */
    readonly costs: readonly StepCost[];
}

/** The runner playing scenarios through the driver of each one's interaction. */
export const Runner = {
    /** Play a scenario some times through new drivers. */
    async measure<Speaks extends Interaction, Instance>(
        scenario: Scenario<Speaks, Instance>,
        type: DriverType<Speaks, Instance>,
        plays = 1,
    ): Promise<Play[]> {
        const played: Play[] = [];
        for (let play = 0; play < plays; play += 1) {
            played.push(await playOnce(scenario, type));
        }

        return played;
    },

    /** Play a scenario as a test through the driver of its interaction, saving its costs. */
    play(scenario: Scenario, types: readonly DriverType[]): void {
        // pick the driver speaking the scenario's interaction
        const name = scenario.interaction.name;
        const type = types.find((candidate) => candidate.interaction.name === name);
        if (type === undefined) {
            throw new TypeError(`no driver plays the ${name} interaction of ${scenario.name}`);
        }

        // save the costs of its plays, then expect the observations it names
        const then = scenario.then;
        test(scenario.name, async () => {
            // play the scenario as often as asked, and save its costs
            const { variables } = testHost().process;
            const plays = Number(variables[PLAYS_VARIABLE] ?? 1);
            const played = await Runner.measure(scenario, type, plays);
            await Runner.save(scenario, played, variables[BASELINE_VARIABLE] ?? CURRENT_BASELINE);

            // expect every play to observe the same, as a world plays alike from its seed
            const expected = "each" in then ? then.each : [then.end];
            expect(played.map((play) => play.observed)).toEqual(played.map(() => expected));
        });
    },

    /** Play each scenario a module exports as a test. */
    playModule(exports: Readonly<Record<string, unknown>>, types: readonly DriverType[]): void {
        for (const exported of Object.values(exports)) {
            if (isScenario(exported)) {
                Runner.play(exported, types);
            }
        }
    },

    /** Save a scenario's measurement under a baseline in Go's benchmark format, for benchstat. */
    async save(scenario: Scenario, played: readonly Play[], baseline: string): Promise<void> {
        // write the lines under the baseline's directory
        const { files, process } = testHost();
        const directory = join(process.workingDirectory, BENCH_DIRECTORY, baseline);
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(
            join(directory, `${scenario.name}.txt`),
            Runner.format(scenario, played),
        );
    },

    /** Write a measurement as Go benchmark lines: one per step and play, each one iteration. */
    format(scenario: Scenario, played: readonly Play[]): string {
        const lines = played.flatMap(({ costs }) =>
            costs.map((cost, index) => {
                // name the step by the scenario, its position and its action
                const step = scenario.when[index];
                const action =
                    typeof step === "object" && step !== null && "action" in step
                        ? String(step.action)
                        : "step";
                const name = `Benchmark${pascal(scenario.name)}/${index + 1}-${action}`;

                // write the host CPU, the world time and each counter per iteration
                const metrics = [
                    `${cost.cpu} cpu-ns/op`,
                    `${cost.world} world-ms/op`,
                    ...Object.entries(cost.counters).map(([unit, value]) => `${value} ${unit}/op`),
                ];

                return `${name} 1 ${metrics.join(" ")}`;
            }),
        );

        return `${lines.join("\n")}\n`;
    },
};

/** Take a scenario's steps through a new driver, reading observations and step costs. */
async function playOnce<Speaks extends Interaction, Instance>(
    scenario: Scenario<Speaks, Instance>,
    type: DriverType<Speaks, Instance>,
): Promise<Play> {
    // start the driver
    const driver = await type.start(scenario.given);
    const { process } = testHost();
    const isEach = "each" in scenario.then;
    const observed: Observations[] = [];
    const costs: StepCost[] = [];
    for (const [index, step] of scenario.when.entries()) {
        // take the step between two readings of the host's CPU, the world's clock and its counters
        const before = {
            usage: process.usage(),
            now: driver.clock.now(),
            counters: await driver.count(),
        };
        await driver.act(step);
        const after = {
            usage: process.usage(),
            now: driver.clock.now(),
            counters: await driver.count(),
        };

        // keep the step's cost
        const cpu = after.usage.user + after.usage.system - before.usage.user - before.usage.system;
        costs.push({
            cpu: Math.round(cpu * NANOSECONDS),
            world: after.now - before.now,
            counters: Object.fromEntries(
                Object.entries(after.counters).map(([unit, value]) => [
                    unit,
                    value - (before.counters[unit] ?? 0),
                ]),
            ),
        });

        // read the observations after each step, or after the last
        if (isEach || index === scenario.when.length - 1) {
            observed.push(await read(driver, scenario.then.observe));
        }
    }

    return { observed, costs };
}

/** Read each named observation through a driver. */
async function read<Speaks extends Interaction>(
    driver: Driver<Speaks>,
    observations: Readonly<Record<string, schema.Infer<Speaks["observation"]>>>,
): Promise<Observations> {
    const observed: Observations = {};
    for (const [name, observation] of Object.entries(observations)) {
        observed[name] = await driver.observe(observation);
    }

    return observed;
}

/** Write a kebab-case name in PascalCase, as Go names benchmarks. */
function pascal(name: string): string {
    return name
        .split("-")
        .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
        .join("");
}

/** Report whether a module export is a scenario. */
function isScenario(value: unknown): value is Scenario {
    return (
        typeof value === "object" &&
        value !== null &&
        "interaction" in value &&
        "given" in value &&
        "when" in value &&
        "then" in value
    );
}
