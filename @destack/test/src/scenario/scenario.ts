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
import { dirname, join } from "node:path";
import { OtlpExport } from "@destack/telemetry/otlp";
import { isDeepStrictEqual } from "node:util";
import { expect, inject, test } from "vitest";
import { testHost } from "../host/index.ts";
import { Recording } from "./recording.ts";

/** The variable naming a run's baseline, the commit by default. */
const BASELINE_VARIABLE = "DESTACK_BENCH_BASELINE";

/** The targets running outside the simulation. */
const EXTERNAL_TARGETS: ReadonlySet<string> = new Set(["postgresql"]);

/** The variable that replays each scenario's seed to check determinism. */
const REPLAY_VARIABLE = "DESTACK_REPLAY";

/** The results directory below the workspace root. */
const BENCH_DIRECTORY = "target/bench";

/** The file that marks a workspace's root. */
const LOCKFILE = "bun.lock";

/** Milliseconds per second. */
const MILLISECONDS = 1000;

/** A driver taking an interaction's steps and reading its observations, after WebDriver. */
export interface Driver<Step = unknown, Observation = unknown> extends AsyncDisposable {
    /** The world's clock. */
    readonly clock: Clock;
    /** Take one step until its work settles. */
    act(step: SetStep | Step): void | Promise<void>;
    /** Read one observation. */
    observe(observation: Observation): JsonValue | Promise<JsonValue>;
    /** The world's telemetry. */
    readonly telemetry: SimulatedTelemetry;
    /** Read the fingerprint of what the world did, equal across plays of one seed. */
    readonly fingerprint: () => number;
}

/** The drivers of one interaction. */
export interface DriverType<Step = unknown, Observation = unknown, Instance = unknown> {
    /** The interaction the drivers speak. */
    readonly interaction: Interaction<Step, Observation>;
    /** The settings that change the numbers, such as durability. */
    readonly configuration: Configuration;
    /** Start a driver on the examples a scenario is given. */
    start(given: Given<Instance>): Driver<Step, Observation> | Promise<Driver<Step, Observation>>;
}

/** A world's counters by name since it started. */
export type Counters = Readonly<Record<string, number>>;

/** A play of a scenario. */
export interface Playthrough {
    /** The checks the play failed. */
    readonly failures: readonly string[];
    /** Each operation's measurement. */
    readonly measurements: readonly Measurement[];
    /** The world's fingerprint after each step. */
    readonly fingerprints: readonly number[];
    /** The undeclared error records of the play. */
    readonly errors: readonly string[];
    /** The world's telemetry as OTLP export requests, when benched. */
    readonly telemetry: readonly OtlpExport[];
}

/** A benchmark's configuration, as Go's `key: value` lines. */
export type Configuration = Readonly<Record<string, string>>;

/** One operation's measurement. */
export interface Measurement {
    /** The operation, such as `issue.create`, none for an unnamed step. */
    readonly operation: string | undefined;
    /** The repetitions taken. */
    readonly times: number;
    /** When the first repetition started, in Unix milliseconds. */
    readonly startedAt: number;
    /** Each repetition's host time, in seconds. */
    readonly host: readonly number[];
    /** The repetitions' host CPU time, in seconds. */
    readonly cpu: number;
    /** The repetitions' world time, in seconds. */
    readonly world: number;
    /** Each counter's growth, by name. */
    readonly counters: Counters;
    /** The operation's failed checks. */
    readonly failures: readonly string[];
}

/** Play scenarios and measure their operations. */
export const Runner = {
    /** Play a scenario through a new driver, measuring its operations. */
    async playthrough(scenario: Scenario, type: DriverType): Promise<Playthrough> {
        // start the driver
        const { clock, process } = testHost();
        const started = clock.now();
        await using driver = await type.start(scenario.given);
        report(scenario, "start", started);

        // take each step, measuring operations and checking observations
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

        // check the end and the play's telemetry, then read the error records
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

        // dump the world's telemetry when benched
        const isBenched = inject("tier") === "bench";
        const telemetry = isBenched
            ? [
                  OtlpExport.traces(driver.telemetry.spans),
                  OtlpExport.logs(driver.telemetry.logs),
                  ...(await driver.telemetry.metrics()).map((each) => OtlpExport.metrics(each)),
              ]
            : [];

        return { failures, measurements, fingerprints, errors, telemetry };
    },

    /** Play a scenario as a test. */
    play(scenario: Scenario, types: readonly DriverType[]): void {
        const type = Runner.driverOf(scenario.interaction, scenario.name, types);
        test(scenario.name, async () => {
            // expect every check to pass
            const first = await Runner.playthrough(scenario, type);
            expect({ failures: first.failures, errors: first.errors }).toEqual({
                failures: [],
                errors: [],
            });

            // replay the seed when asked
            const { variables } = testHost().process;
            if (variables[REPLAY_VARIABLE] === "1") {
                const second = await Runner.playthrough(scenario, type);
                expect(Runner.checkDeterminism(scenario, first, second)).toBeUndefined();
            }

            // save the results when benched
            if (inject("tier") === "bench") {
                const configuration = await Runner.configuration(scenario, type);
                const baseline = variables[BASELINE_VARIABLE] ?? String(configuration["commit"]);
                const named = first.measurements.filter((each) => each.operation !== undefined);
                await Runner.save(scenario, named, first.telemetry, configuration, baseline);
            }
        });
    },

    /** Resolve a scenario's parameter references. */
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

    /** Name the first step two plays differ after, none when they agree. */
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

    /** Select the driver types among a module's exports. */
    driversOf(exports: Readonly<Record<string, unknown>>): DriverType[] {
        return Object.values(exports).filter((value) => isDriverType(value));
    },

    /** Play each scenario a module exports, as tests. */
    playModule(exports: Readonly<Record<string, unknown>>, types: readonly DriverType[]): void {
        for (const exported of Object.values(exports).filter((each) => isScenario(each))) {
            Runner.play(exported, types);
        }
    },

    /** Pick the driver type of an interaction. */
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

    /** Read a scenario's configuration. */
    async configuration(scenario: Scenario, type: DriverType): Promise<Configuration> {
        // read the commit, marked dirty for a changed tree
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

    /** Save a scenario's measurements and telemetry. */
    async save(
        scenario: Scenario,
        measured: readonly Measurement[],
        telemetry: readonly OtlpExport[],
        configuration: Configuration,
        baseline: string,
    ): Promise<void> {
        // write the measurements and the telemetry below the baseline
        const { files, process } = testHost();
        let root = process.workingDirectory;
        while (!(await files.exists(join(root, LOCKFILE))) && dirname(root) !== root) {
            root = dirname(root);
        }
        const directory = join(root, BENCH_DIRECTORY, baseline, scenario.package.name);
        const name = [scenario.name, ...Object.values(inject("targets") ?? {})].join("-");
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(
            join(directory, `${name}.txt`),
            Runner.format(scenario, measured, configuration),
        );
        await files.writeTextFile(
            join(directory, `${name}.otlp.jsonl`),
            telemetry.map((each) => `${JSON.stringify(each.body)}\n`).join(""),
        );
    },

    /** Write results in Go's benchmark format. */
    format(
        scenario: Scenario,
        measured: readonly Measurement[],
        configuration: Configuration,
    ): string {
        // write the configuration
        const header = Object.entries(configuration).map(([key, value]) => `${key}: ${value}`);

        // write a line per operation
        const lines = measured.map((measurement) => {
            // read the measurement
            const { operation, times, startedAt, host, cpu, world, counters } = measurement;

            // write the times, quantiles and counters
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
                `${startedAt} start-unix-ms`,
                ...Object.entries(counters).map(([unit, value]) => `${value / times} ${unit}/op`),
            ];

            return `${name} ${times} ${values.join(" ")}`;
        });

        return `${[...header, "", ...lines].join("\n")}\n`;
    },
};

/** Read a quantile of sorted values by its nearest rank. */
function quantile(sorted: readonly number[], fraction: number): number {
    // take the value at the fraction's rank
    const rank = Math.min(sorted.length - 1, Math.ceil(fraction * sorted.length) - 1);

    return present(sorted[Math.max(0, rank)], "a measured repetition");
}

/** Print a step's host time on standard error. */
function report(scenario: Scenario, step: string, started: number): void {
    const { clock, process } = testHost();
    const seconds = ((clock.now() - started) / MILLISECONDS).toFixed(1);
    process.stderr.writeSync(`${scenario.name}: ${step} ${seconds} s\n`);
}

/** Measure an operation's repetitions. */
async function measure(
    driver: Driver,
    operation: Operation,
    usage: () => { readonly user: number; readonly system: number },
): Promise<Measurement> {
    // warm up, then time each repetition
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
    const startedAt = testHost().clock.now();
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

    // check metric growth only in a seeded world
    const isSeeded = Object.values(inject("targets") ?? {}).every(
        (target) => !EXTERNAL_TARGETS.has(target),
    );
    const checks = (operation.then ?? []).filter((check) => isSeeded || !("metric" in check));

    // record the totals and the failed checks
    const cpu = after.usage.user + after.usage.system - before.usage.user - before.usage.system;
    const emitted = after.recorded.since(before.recorded);

    return {
        operation: operation.operation,
        times,
        startedAt,
        host,
        cpu,
        world: (after.now - before.now) / MILLISECONDS,
        counters: {
            ...emitted.counters(),
            ...emitted.work(),
            ...emitted.queries(),
            ...emitted.rows(),
            ...emitted.reads(),
            ...emitted.wakes(),
            ...emitted.listings(),
        },
        failures: emitted.failures(labelOf(operation), times, checks),
    };
}

/** List the failed observation checks. */
async function observe(driver: Driver, checks: readonly Check[], label: string): Promise<string[]> {
    const failures: string[] = [];
    for (const check of checks) {
        // compare the observation with the expected value
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

/** Name a step by its operation, else its JSON. */
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

/** Report whether a value is a driver type. */
function isDriverType(value: unknown): value is DriverType {
    return (
        (typeof value === "object" || typeof value === "function") &&
        value !== null &&
        "interaction" in value &&
        "configuration" in value &&
        "start" in value &&
        typeof value.start === "function"
    );
}
