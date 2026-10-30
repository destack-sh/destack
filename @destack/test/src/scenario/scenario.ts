import type { Benchmark, Given, Interaction, Scenario, SetStep } from "@destack/package/declare";
import type { Clock } from "@destack/environment";
import type { JsonValue, schema } from "@destack/schema";
import { join } from "node:path";
import { expect, inject, test } from "vitest";
import { testHost } from "../host/index.ts";

/** The variable naming the baseline a run's results are saved as, `current` by default. */
const BASELINE_VARIABLE = "DESTACK_BENCH_BASELINE";

/** The baseline a run's results are saved as without one named. */
const CURRENT_BASELINE = "current";

/** The directory of the saved results, below the package's working directory. */
const BENCH_DIRECTORY = "node_modules/.cache/destack/bench";

/** The most milliseconds a benchmark takes, its world's setup included. */
const BENCHMARK_MILLISECONDS = 600_000;

/** Milliseconds per second: clocks read milliseconds, and Go's format writes seconds. */
const MILLISECONDS = 1000;

/** A driver that takes an interaction's steps and reads its observations, after WebDriver. */
export interface Driver<Speaks extends Interaction = Interaction> extends AsyncDisposable {
    /** The clock the driver's world runs on, which measures each operation's world time. */
    readonly clock: Clock;
    /** Take one step. */
    act(step: SetStep | schema.Infer<Speaks["step"]>): void | Promise<void>;
    /** Read one observation. */
    observe(observation: schema.Infer<Speaks["observation"]>): JsonValue | Promise<JsonValue>;
    /** Read the world's counters since the driver started, by name. */
    readonly count: () => Counters | Promise<Counters>;
    /** Read the fingerprint of everything the world did so far, equal across plays of one seed. */
    readonly fingerprint: () => number;
}

/** The drivers of one interaction, each started on the examples a scenario is given. */
export interface DriverType<Speaks extends Interaction = Interaction, Instance = unknown> {
    /** The interaction the drivers speak. */
    readonly interaction: Speaks;
    /** Start a driver on the examples a scenario is given, none for a benchmark. */
    start(given: Given<Instance>): Driver<Speaks> | Promise<Driver<Speaks>>;
}

/** The values of a scenario's named observations at one point. */
export type Observations = Record<string, JsonValue>;

/** A world's counters by name, such as `db.client.operation.duration.count`, since it started. */
export type Counters = Readonly<Record<string, number>>;

/** A world's fingerprint after one step of a play. */
export interface StepFingerprint {
    /** The step, as JSON. */
    readonly step: string;
    /** The world's fingerprint after it. */
    readonly fingerprint: number;
}

/** A play of a world: its result and the world's fingerprint after each step. */
export interface Playthrough<Result> {
    /** The play's result. */
    readonly result: Result;
    /** The world's fingerprint after each step. */
    readonly steps: readonly StepFingerprint[];
}

/** The configuration a benchmark ran in, written as Go's `key: value` lines. */
export type Configuration = Readonly<Record<string, string>>;

/** One operation's measurement: its iterations' host CPU, world time and counter growth. */
export interface Measurement {
    /** The operation, such as `issue.create`. */
    readonly operation: string;
    /** The iterations taken. */
    readonly iterations: number;
    /** The host CPU time all iterations took, in seconds. */
    readonly cpu: number;
    /** The world time all iterations took on the driver's clock, in seconds. */
    readonly world: number;
    /** Each counter's growth over all iterations, by name. */
    readonly counters: Counters;
}

/** The runner playing scenarios and measuring benchmarks through their interactions' drivers. */
export const Runner = {
    /** Play a scenario through a new driver, reading its observations. */
    async observe<Speaks extends Interaction, Instance>(
        scenario: Scenario<Speaks, Instance>,
        type: DriverType<Speaks, Instance>,
    ): Promise<Playthrough<Observations[]>> {
        // take each step, reading the observations after each step or after the last
        await using driver = await type.start(scenario.given);
        const isEach = "each" in scenario.then;
        const observed: Observations[] = [];
        const steps: StepFingerprint[] = [];
        for (const [index, step] of scenario.when.entries()) {
            await driver.act(step);
            steps.push({ step: JSON.stringify(step), fingerprint: driver.fingerprint() });
            if (isEach || index === scenario.when.length - 1) {
                observed.push(await read(driver, scenario.then.observe));
            }
        }

        return { result: observed, steps };
    },

    /** Set a benchmark's world up through a new driver, then measure each operation in order. */
    async measure<Speaks extends Interaction>(
        benchmark: Benchmark<Speaks>,
        type: DriverType<Speaks>,
    ): Promise<Playthrough<Measurement[]>> {
        // set the world up unmeasured, reporting each step's host time as it ends
        const { clock, process } = testHost();
        const started = clock.now();
        await using driver = await type.start({ examples: [] });
        report(benchmark, "start", started);
        const steps: StepFingerprint[] = [];
        for (const step of benchmark.setup) {
            const taken = clock.now();
            await driver.act(step);
            report(benchmark, JSON.stringify(step), taken);
            steps.push({ step: JSON.stringify(step), fingerprint: driver.fingerprint() });
        }

        // take each operation's iterations between two readings of CPU, clock and counters
        const measured: Measurement[] = [];
        for (const [operation, step] of Object.entries(benchmark.operations)) {
            const before = { usage: process.usage(), now: driver.clock.now(), taken: clock.now() };
            const counted = await driver.count();
            for (let iteration = 0; iteration < benchmark.iterations; iteration += 1) {
                await driver.act(step);
            }
            const after = { usage: process.usage(), now: driver.clock.now() };
            report(benchmark, `${operation} x${benchmark.iterations}`, before.taken);
            steps.push({ step: operation, fingerprint: driver.fingerprint() });
            const counters = await driver.count();

            // record the operation's totals
            const cpu =
                after.usage.user + after.usage.system - before.usage.user - before.usage.system;
            const grown = Object.entries(counters).map(
                ([name, value]) => [name, value - (counted[name] ?? 0)] as const,
            );
            measured.push({
                operation,
                iterations: benchmark.iterations,
                cpu,
                world: (after.now - before.now) / MILLISECONDS,
                counters: Object.fromEntries(grown),
            });
        }

        return { result: measured, steps };
    },

    /** Play a world twice from one seed, refusing a replay whose world diverges. */
    async replay<Result>(play: () => Promise<Playthrough<Result>>): Promise<Result> {
        // play twice
        const first = await play();
        const second = await play();

        // refuse the first step after which the two worlds differ
        for (const [index, { step, fingerprint }] of first.steps.entries()) {
            if (second.steps[index]?.fingerprint !== fingerprint) {
                throw new Error(
                    `the world diverged between two plays of one seed at step ${index}: ${step}`,
                );
            }
        }

        return first.result;
    },

    /** Play a scenario as a test through the driver of its interaction. */
    play(scenario: Scenario, types: readonly DriverType[]): void {
        const type = Runner.driverOf(scenario.interaction, scenario.name, types);
        test(scenario.name, async () => {
            // expect each step's observations, or the last one's
            const then = scenario.then;
            const expected = "each" in then ? then.each : [then.end];

            // play the scenario twice, expecting its observations from one seed
            const play = () => Runner.observe(scenario, type);
            expect(await Runner.replay(play)).toEqual(expected);
        });
    },

    /** Measure a benchmark as a test, saving its results and expecting operations within limits. */
    bench(benchmark: Benchmark, types: readonly DriverType[]): void {
        const type = Runner.driverOf(benchmark.interaction, benchmark.name, types);
        test(`measure ${benchmark.description}`, { timeout: BENCHMARK_MILLISECONDS }, async () => {
            // measure, then save the results under the run's baseline
            const play = () => Runner.measure(benchmark, type);
            const measured = await Runner.replay(play);
            const { variables } = testHost().process;
            const baseline = variables[BASELINE_VARIABLE] ?? CURRENT_BASELINE;
            const configuration = await Runner.configuration(benchmark);
            await Runner.save(benchmark, measured, configuration, baseline);

            // expect each operation's counters within their limits, refusing a limit nothing counts
            const exceeded = measured.flatMap(({ operation, iterations, counters }) =>
                Object.entries(benchmark.limits[operation] ?? {}).flatMap(([name, limit]) => {
                    const total = counters[name];
                    // refuse a counter the world never counted
                    if (total === undefined) {
                        return [`${operation} ${name}: not counted`];
                    }
                    // compare the counter per iteration otherwise
                    else {
                        const each = total / iterations;

                        return each > limit ? [`${operation} ${name}: ${each} > ${limit}`] : [];
                    }
                }),
            );
            expect(exceeded).toEqual([]);
        });
    },

    /** Play each scenario and measure each benchmark a module exports, as tests. */
    playModule(exports: Readonly<Record<string, unknown>>, types: readonly DriverType[]): void {
        for (const exported of Object.values(exports)) {
            // play a scenario
            if (isScenario(exported)) {
                Runner.play(exported, types);
            }
            // measure a benchmark
            else if (isBenchmark(exported)) {
                Runner.bench(exported, types);
            }
        }
    },

    /** Pick the driver type speaking an interaction, refusing one no driver speaks. */
    driverOf<Type extends DriverType>(
        interaction: Interaction,
        name: string,
        types: readonly Type[],
    ): Type {
        const type = types.find((candidate) => candidate.interaction.name === interaction.name);
        if (type === undefined) {
            throw new TypeError(`no driver plays the ${interaction.name} interaction of ${name}`);
        }

        return type;
    },

    /** Read the configuration a benchmark runs in: its package, targets, host and commit. */
    async configuration(benchmark: Benchmark): Promise<Configuration> {
        // read the commit the working tree is at, marked dirty when it has changes
        const { host, process } = testHost();
        const output = async (command: readonly string[]) => {
            const { stdout } = await process.spawn(command).output();

            return new TextDecoder().decode(stdout).trim();
        };
        const commit = await output(["git", "rev-parse", "--short", "HEAD"]);
        const isDirty = (await output(["git", "status", "--porcelain"])) !== "";

        return {
            pkg: benchmark.package.name,
            ...inject("targets"),
            os: host.os,
            arch: host.architecture,
            commit: isDirty ? `${commit}-dirty` : commit,
        };
    },

    /** Save a benchmark's results under a baseline in Go's benchmark format, for benchstat. */
    async save(
        benchmark: Benchmark,
        measured: readonly Measurement[],
        configuration: Configuration,
        baseline: string,
    ): Promise<void> {
        // write one file per benchmark and target combination under the baseline's directory
        const { files, process } = testHost();
        const directory = join(process.workingDirectory, BENCH_DIRECTORY, baseline);
        const targets = Object.values(inject("targets") ?? {});
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(
            join(directory, `${[benchmark.name, ...targets].join("-")}.txt`),
            Runner.format(benchmark, measured, configuration),
        );
    },

    /** Write results in Go's benchmark format: configuration lines, then one line per operation. */
    format(
        benchmark: Benchmark,
        measured: readonly Measurement[],
        configuration: Configuration,
    ): string {
        // write the configuration as key: value lines
        const header = Object.entries(configuration).map(([key, value]) => `${key}: ${value}`);

        // name each line by family, operation and parameters, its values per operation
        const parameters = Object.entries(benchmark.parameters)
            .map(([key, value]) => `/${key}=${value}`)
            .join("");
        const lines = measured.map(({ operation, iterations, cpu, world, counters }) => {
            const name = `Benchmark${pascal(benchmark.name)}/op=${operation}${parameters}`;
            const values = [
                `${world / iterations} sec/op`,
                `${cpu / iterations} cpu-sec/op`,
                ...Object.entries(counters).map(
                    ([unit, value]) => `${value / iterations} ${unit}/op`,
                ),
            ];

            return `${name} ${iterations} ${values.join(" ")}`;
        });

        return `${[...header, "", ...lines].join("\n")}\n`;
    },
};

/** Report a benchmark step's host time on standard error, as a long setup runs. */
function report(benchmark: Benchmark, step: string, started: number): void {
    const { clock, process } = testHost();
    const seconds = ((clock.now() - started) / MILLISECONDS).toFixed(1);
    process.stderr.writeSync(`${benchmark.name}: ${step} ${seconds} s\n`);
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

/** Report whether a module export is a benchmark. */
function isBenchmark(value: unknown): value is Benchmark {
    return (
        typeof value === "object" &&
        value !== null &&
        "interaction" in value &&
        "setup" in value &&
        "operations" in value
    );
}
