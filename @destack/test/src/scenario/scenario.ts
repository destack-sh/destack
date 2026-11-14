import {
    Operation,
    Reference,
    Check,
    type Given,
    type Interaction,
    type Scenario,
    type SetStep,
} from "@destack/package/declare";
import { Clock } from "@destack/environment";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import { type JsonValue, present, schema } from "@destack/schema";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { expect, inject, test } from "vitest";
import { testHost } from "../host/index.ts";
import { Recording } from "./recording.ts";

/** The variable naming the baseline a run's results are saved as, `current` by default. */
const BASELINE_VARIABLE = "DESTACK_BENCH_BASELINE";

/** The variable that replays each scenario's seed, as `1`, to check its world is deterministic. */
const REPLAY_VARIABLE = "DESTACK_REPLAY";

/** The baseline a run's results are saved as without one named. */
const CURRENT_BASELINE = "current";

/** The directory of the saved results, below the package's working directory. */
const BENCH_DIRECTORY = "node_modules/.cache/destack/bench";

/** Milliseconds per second: clocks read milliseconds, and Go's format writes seconds. */
const MILLISECONDS = 1000;

/** A driver that takes an interaction's steps and reads its observations, after WebDriver. */
export interface Driver<Step = unknown, Observation = unknown> extends AsyncDisposable {
    /** The clock the driver's world runs on, which measures each operation's world time. */
    readonly clock: Clock;
    /** Take one step, until the work it started settles. */
    act(step: SetStep | Step): void | Promise<void>;
    /** Read one observation. */
    observe(observation: Observation): JsonValue | Promise<JsonValue>;
    /** The telemetry the driver's world records, which operations are checked and measured by. */
    readonly telemetry: SimulatedTelemetry;
    /** Read the fingerprint of everything the world did so far, equal across plays of one seed. */
    readonly fingerprint: () => number;
}

/** The drivers of one interaction, each started on the examples a scenario is given. */
export interface DriverType<Step = unknown, Observation = unknown, Instance = unknown> {
    /** The interaction the drivers speak. */
    readonly interaction: Interaction<Step, Observation>;
    /** The settings the drivers' worlds run with that change their numbers, such as durability. */
    readonly configuration: Configuration;
    /** Start a driver on the examples a scenario is given. */
    start(given: Given<Instance>): Driver<Step, Observation> | Promise<Driver<Step, Observation>>;
}

/** A world's counters by name, such as `db.client.operation.duration.count`, since it started. */
export type Counters = Readonly<Record<string, number>>;

/** A play of a scenario: its failed checks, measurements and world fingerprints. */
export interface Playthrough {
    /** The checks the play failed, each step's and the end's. */
    readonly failures: readonly string[];
    /** Each operation's measurement. */
    readonly measurements: readonly Measurement[];
    /** The world's fingerprint after each step. */
    readonly fingerprints: readonly number[];
    /** The error records any host logged in the whole play that no check declares. */
    readonly errors: readonly string[];
}

/** The configuration a benchmark ran in, written as Go's `key: value` lines. */
export type Configuration = Readonly<Record<string, string>>;

/** One operation's measurement: its repetitions' host CPU, world time and counter growth. */
export interface Measurement {
    /** The operation, such as `issue.create`, none for a checked step without a name. */
    readonly operation: string | undefined;
    /** The repetitions taken. */
    readonly times: number;
    /** The host time each repetition took, in seconds, in order. */
    readonly host: readonly number[];
    /** The host CPU time all repetitions took, in seconds. */
    readonly cpu: number;
    /** The world time all repetitions took on the driver's clock, in seconds. */
    readonly world: number;
    /** Each counter's growth over all repetitions, by name. */
    readonly counters: Counters;
    /** The checks of the operation its repetitions fail. */
    readonly failures: readonly string[];
}

/** The runner playing scenarios and measuring benchmarks through their interactions' drivers. */
export const Runner = {
    /** Play a scenario through a new driver, measuring its operations. */
    async playthrough(scenario: Scenario, type: DriverType): Promise<Playthrough> {
        // start the driver, reporting its start's host time
        const { clock, process } = testHost();
        const started = clock.now();
        await using driver = await type.start(scenario.given);
        report(scenario, "start", started);

        // take each step, measuring a step with checks over its repetitions, then checking its observations
        const { when, then } = Runner.resolve(scenario);
        const start = await Recording.read(driver.telemetry);
        const failures: string[] = [];
        const measurements: Measurement[] = [];
        const fingerprints: number[] = [];
        for (const entry of when) {
            const taken = clock.now();
            if (Operation.is(entry)) {
                const measurement = await measure(driver, entry, process.usage);
                failures.push(
                    ...measurement.failures,
                    ...(await observe(driver, entry.then ?? [], labelOf(entry))),
                );
                measurements.push(measurement);
            } else {
                await driver.act(Reference.repeat(entry, 0));
            }
            report(scenario, labelOf(entry), taken);
            fingerprints.push(driver.fingerprint());
        }

        // check the end's observations and the whole play's telemetry, then read every error record
        const played = (await Recording.read(driver.telemetry)).since(start);
        failures.push(
            ...(await observe(driver, then, "the end")),
            ...played.failures("the play", 1, then),
        );
        const declared = [
            ...then,
            ...when.flatMap((entry) => (Operation.is(entry) ? (entry.then ?? []) : [])),
        ];
        const errors = played.undeclaredErrors(declared);

        return { failures, measurements, fingerprints, errors };
    },

    /** Play a scenario as a test, expecting its checks, replaying its seed when asked. */
    play(scenario: Scenario, types: readonly DriverType[]): void {
        const type = Runner.driverOf(scenario.interaction, scenario.name, types);
        test(scenario.name, async () => {
            // expect every check to pass with no undeclared error
            const first = await Runner.playthrough(scenario, type);
            expect({ failures: first.failures, errors: first.errors }).toEqual({
                failures: [],
                errors: [],
            });

            // replay the seed when asked, expecting the same world after every step
            const { variables } = testHost().process;
            if (variables[REPLAY_VARIABLE] === "1") {
                const second = await Runner.playthrough(scenario, type);
                expect(Runner.checkDeterminism(scenario, first, second)).toBeUndefined();
            }

            // save the measurements under the run's baseline when benched
            if (inject("tier") === "bench") {
                const baseline = variables[BASELINE_VARIABLE] ?? CURRENT_BASELINE;
                const configuration = await Runner.configuration(scenario, type);
                const named = first.measurements.filter((each) => each.operation !== undefined);
                await Runner.save(scenario, named, configuration, baseline);
            }
        });
    },

    /** Resolve a scenario's references to its parameters in its steps and end checks. */
    resolve(scenario: Scenario): {
        readonly when: readonly unknown[];
        readonly then: readonly Check[];
    } {
        // resolve the steps, then the end's checks
        const { parameters } = scenario;
        const when = scenario.when.map((step) => Reference.resolve(step, parameters));
        const then = schema
            .array(Check)
            .parse((scenario.then ?? []).map((check) => Reference.resolve(check, parameters)));

        return { when, then };
    },

    /** Name the first step after which two plays of a scenario differ, none when they agree. */
    checkDeterminism(
        scenario: Scenario,
        first: Playthrough,
        second: Playthrough,
    ): string | undefined {
        const index = first.fingerprints.findIndex(
            (fingerprint, at) => second.fingerprints[at] !== fingerprint,
        );
        const entry = Runner.resolve(scenario).when[index];

        return entry === undefined ? undefined : `step ${index}: ${labelOf(entry)}`;
    },

    /** Play each scenario a module exports, as tests. */
    playModule(exports: Readonly<Record<string, unknown>>, types: readonly DriverType[]): void {
        for (const exported of Object.values(exports).filter((each) => isScenario(each))) {
            Runner.play(exported, types);
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
            throw new TypeError(
                `no driver plays the '${interaction.name}' interaction of '${name}'`,
            );
        }

        return type;
    },

    /** Read a scenario's configuration: its package, targets, driver, host and commit. */
    async configuration(scenario: Scenario, type: DriverType): Promise<Configuration> {
        // read the commit the working tree is at, marked dirty when it has changes
        const { host, process } = testHost();
        const output = async (command: readonly string[]) => {
            const { stdout } = await process.spawn(command).output();

            return new TextDecoder().decode(stdout).trim();
        };
        const commit = await output(["git", "rev-parse", "--short", "HEAD"]);
        const isDirty = (await output(["git", "status", "--porcelain"])) !== "";

        return {
            pkg: scenario.package.name,
            ...inject("targets"),
            ...type.configuration,
            os: host.os,
            arch: host.architecture,
            commit: isDirty ? `${commit}-dirty` : commit,
        };
    },

    /** Save a scenario's measurements under a baseline in Go's benchmark format, for benchstat. */
    async save(
        scenario: Scenario,
        measured: readonly Measurement[],
        configuration: Configuration,
        baseline: string,
    ): Promise<void> {
        // write one file per scenario and target combination under the baseline's directory
        const { files, process } = testHost();
        const directory = join(process.workingDirectory, BENCH_DIRECTORY, baseline);
        const targets = Object.values(inject("targets") ?? {});
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(
            join(directory, `${[scenario.name, ...targets].join("-")}.txt`),
            Runner.format(scenario, measured, configuration),
        );
    },

    /** Write results in Go's benchmark format: configuration lines, then one line per operation. */
    format(
        scenario: Scenario,
        measured: readonly Measurement[],
        configuration: Configuration,
    ): string {
        // write the configuration as key: value lines
        const header = Object.entries(configuration).map(([key, value]) => `${key}: ${value}`);

        // name each line by scenario and operation, its values per repetition
        const lines = measured.map(({ operation, times, host, cpu, world, counters }) => {
            // write host time per repetition and its quantiles, CPU, world time and each counter
            const parameters = Object.entries(scenario.parameters).map(
                ([key, value]) => `/${key}=${value}`,
            );
            const name = `Benchmark${pascal(scenario.name)}${parameters.join("")}/op=${operation}`;
            const ordered = host.toSorted((left, right) => left - right);
            const total = host.reduce((sum, seconds) => sum + seconds, 0);
            const values = [
                `${total / times} sec/op`,
                `${quantile(ordered, 0.5)} p50-sec/op`,
                `${quantile(ordered, 0.95)} p95-sec/op`,
                `${cpu / times} cpu-sec/op`,
                `${world / times} world-sec/op`,
                ...Object.entries(counters).map(([unit, value]) => `${value / times} ${unit}/op`),
            ];

            return `${name} ${times} ${values.join(" ")}`;
        });

        return `${[...header, "", ...lines].join("\n")}\n`;
    },
};

/** Read a quantile of sorted values by its nearest rank. */
function quantile(sorted: readonly number[], fraction: number): number {
    // take the value at the fraction's rank, the first for a fraction below one value
    const rank = Math.min(sorted.length - 1, Math.ceil(fraction * sorted.length) - 1);

    return present(sorted[Math.max(0, rank)], "a measured repetition");
}

/** Report a scenario step's host time on standard error, as a long scenario runs. */
function report(scenario: Scenario, step: string, started: number): void {
    const { clock, process } = testHost();
    const seconds = ((clock.now() - started) / MILLISECONDS).toFixed(1);
    process.stderr.writeSync(`${scenario.name}: ${step} ${seconds} s\n`);
}

/** Measure an operation's repetitions between two readings of CPU, world time and counters. */
async function measure(
    driver: Driver,
    operation: Operation,
    usage: () => { readonly user: number; readonly system: number },
): Promise<Measurement> {
    // warm the step up, then take each repetition, timing it on the host, numbering them on
    const warmup = operation.warmup ?? 0;
    for (let repetition = 0; repetition < warmup; repetition += 1) {
        await driver.act(Reference.repeat(operation.step, repetition));
    }
    const before = {
        usage: usage(),
        now: driver.clock.now(),
        recorded: await Recording.read(driver.telemetry),
    };
    const host: number[] = [];
    const times = operation.times ?? 1;
    for (let repetition = 0; repetition < times; repetition += 1) {
        const acted = Clock.monotonic.now();
        await driver.act(Reference.repeat(operation.step, warmup + repetition));
        host.push((Clock.monotonic.now() - acted) / MILLISECONDS);
    }
    const after = {
        usage: usage(),
        now: driver.clock.now(),
        recorded: await Recording.read(driver.telemetry),
    };

    // record the repetitions' totals and the checks their telemetry fails
    const cpu = after.usage.user + after.usage.system - before.usage.user - before.usage.system;
    const emitted = after.recorded.since(before.recorded);

    return {
        operation: operation.operation,
        times,
        host,
        cpu,
        world: (after.now - before.now) / MILLISECONDS,
        counters: emitted.counters(),
        failures: emitted.failures(labelOf(operation), times, operation.then ?? []),
    };
}

/** List the observation checks a driver's world fails, each with what it read. */
async function observe(driver: Driver, checks: readonly Check[], label: string): Promise<string[]> {
    const failures: string[] = [];
    for (const check of checks) {
        // read the observation and compare it with the expected value
        if ("observe" in check) {
            const value = await driver.observe(check.observe);
            if (!isDeepStrictEqual(value, check.is)) {
                failures.push(
                    `${label} observed ${JSON.stringify(check.observe)} as ${JSON.stringify(value)}, not ${JSON.stringify(check.is)}`,
                );
            }
        }
    }

    return failures;
}

/** Name a step as reports and failures do: its operation, else its JSON. */
function labelOf(entry: unknown): string {
    return Operation.is(entry) && entry.operation !== undefined
        ? entry.operation
        : JSON.stringify(Operation.is(entry) ? entry.step : entry);
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
        "parameters" in value
    );
}
