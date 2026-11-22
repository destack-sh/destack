import { basename, join, relative } from "node:path";
import type { FileSystem } from "@destack/environment";

/** The suffix of a scenario's measurements file. */
const MEASUREMENTS = ".txt";

/** The first table column holding numbers. */
const NUMBER_COLUMN = 3;

/** Percent per whole. */
const PERCENT = 100;

/** Milliseconds per second. */
const MILLISECONDS = 1000;

/** One operation's line in Go's benchmark format. */
export class BenchLine {
    /** The package, such as @destack/machine. */
    readonly package: string;
    /** The scenario and its targets, such as create-space-sqlite. */
    readonly scenario: string;
    /** The operation, such as space.create. */
    readonly operation: string;
    /** The repetitions measured. */
    readonly times: number;
    /** The runs measured, each in a fresh world, as Go's -count. */
    readonly runs: number;
    /** The values by unit, such as sec/op. */
    readonly values: ReadonlyMap<string, number>;

    /** Take a line's fields. */
    constructor(
        line: Pick<BenchLine, "package" | "scenario" | "operation" | "times" | "runs" | "values">,
    ) {
        // keep the fields
        this.package = line.package;
        this.scenario = line.scenario;
        this.operation = line.operation;
        this.times = line.times;
        this.runs = line.runs;
        this.values = line.values;
    }

    /** Merge one operation's runs: each unit's median and the repetitions summed, as benchstat. */
    static median(lines: readonly BenchLine[]): BenchLine {
        // take each unit's median across the runs
        const [first] = lines;
        if (first === undefined) {
            throw new RangeError("a median needs at least one run");
        }
        const units = new Set(lines.flatMap((line) => [...line.values.keys()]));
        const values = new Map<string, number>();
        for (const unit of units) {
            const sorted = lines
                .map((line) => line.value(unit))
                .toSorted((left, right) => left - right);
            const middle = Math.floor(sorted.length / 2);
            const median =
                sorted.length % 2 === 1
                    ? (sorted[middle] ?? 0)
                    : ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2;
            values.set(unit, median);
        }

        return new BenchLine({
            package: first.package,
            scenario: first.scenario,
            operation: first.operation,
            times: lines.reduce((total, line) => total + line.times, 0),
            runs: lines.reduce((total, line) => total + line.runs, 0),
            values,
        });
    }

    /** Parse a benchmark line, absent for any other line. */
    static parse(text: string, packageName: string, scenario: string): BenchLine | undefined {
        // split the name, the repetitions and the value pairs
        const [name, times, ...pairs] = text.trim().split(/\s+/u);
        const isBenchmark = name?.startsWith("Benchmark") === true;
        const operation = isBenchmark ? name?.split("/op=")[1] : undefined;
        if (operation === undefined || times === undefined) {
            return undefined;
        }
        const values = new Map<string, number>();
        for (let index = 0; index + 1 < pairs.length; index += 2) {
            values.set(String(pairs[index + 1]), Number(pairs[index]));
        }

        return new BenchLine({
            package: packageName,
            scenario,
            operation,
            times: Number(times),
            runs: 1,
            values,
        });
    }

    /** Read a value by its unit, zero for a missing one. */
    value(unit: string): number {
        return this.values.get(unit) ?? 0;
    }

    /** Report whether another line measures the same operation of the same scenario. */
    isSame(other: BenchLine): boolean {
        return (
            this.package === other.package &&
            this.scenario === other.scenario &&
            this.operation === other.operation
        );
    }

    /** Read the table's numbers: times in milliseconds, then counters. */
    numbers(): number[] {
        return [
            this.value("p50-sec/op") * MILLISECONDS,
            this.value("p95-sec/op") * MILLISECONDS,
            this.value("world-sec/op") * MILLISECONDS,
            this.sum((unit) => unit.startsWith("queries{")),
            this.sum((unit) => /^rows\{[A-Z]+\}\/op$/u.test(unit)),
            this.value("queries{TRANSACTION}/op"),
        ];
    }

    /** Sum the values of matching units. */
    sum(matches: (unit: string) => boolean): number {
        return [...this.values].reduce(
            (total, [unit, each]) => total + (matches(unit) ? each : 0),
            0,
        );
    }
}

/** A bench run: the lines of one baseline directory. */
export class BenchRun {
    /** The baseline, such as a commit. */
    readonly baseline: string;
    /** The lines. */
    readonly lines: readonly BenchLine[];

    /** Take a run's baseline and lines. */
    constructor(baseline: string, lines: readonly BenchLine[]) {
        this.baseline = baseline;
        this.lines = lines;
    }

    /** Read a baseline directory's measurements files. */
    static async read(directory: string, files: FileSystem): Promise<BenchRun> {
        // parse each measurements file
        const entries = await files.readDirectory(directory, { recursive: true });
        const lines: BenchLine[] = [];
        for (const entry of entries.filter((each) => each.name.endsWith(MEASUREMENTS))) {
            const path = join(directory, entry.name);
            const packageName = relative(directory, join(path, ".."));
            const scenario = basename(entry.name, MEASUREMENTS);
            const text = await files.readTextFile(path);
            for (const line of text.split("\n")) {
                const parsed = BenchLine.parse(line, packageName, scenario);
                if (parsed !== undefined) {
                    lines.push(parsed);
                }
            }
        }

        // merge each operation's runs into their medians
        const runs = new Map<string, BenchLine[]>();
        for (const line of lines) {
            const key = [line.package, line.scenario, line.operation].join(" ");
            runs.set(key, [...(runs.get(key) ?? []), line]);
        }

        return new BenchRun(
            basename(directory),
            [...runs.values()].map((each) => BenchLine.median(each)),
        );
    }

    /** Write the run as a table, slowest first, against a base run when given. */
    table(base?: BenchRun): string {
        // read each line's times and counters, and its base line's
        const header = [
            "operation",
            "scenario",
            "package",
            "n",
            "runs",
            "p50 ms",
            "p95 ms",
            "world ms",
        ];
        const work = ["queries", "rows", "tx"];
        const rows = this.lines
            .toSorted((left, right) => right.value("p50-sec/op") - left.value("p50-sec/op"))
            .map((line) => {
                // compare with the base's line of the same operation
                const before = base?.lines.find((each) => each.isSame(line));
                const numbers = line.numbers();
                const baseNumbers = before?.numbers();

                return [
                    line.operation,
                    line.scenario,
                    line.package,
                    String(line.times),
                    String(line.runs),
                    ...numbers.map((value, index) => change(value, baseNumbers?.[index])),
                ];
            });

        // pad each column, numbers to the right
        const table = [[...header, ...work], ...rows];
        const widths = table[0]?.map((_, column) =>
            Math.max(...table.map((row) => row[column]?.length ?? 0)),
        );

        return table
            .map((row) =>
                row
                    .map((cell, column) => {
                        const width = widths?.[column] ?? 0;

                        return column >= NUMBER_COLUMN ? cell.padStart(width) : cell.padEnd(width);
                    })
                    .join("  "),
            )
            .join("\n");
    }
}

/** Write a number rounded, with its change against a base number when given. */
function change(value: number, base: number | undefined): string {
    // round it, alone without a base
    const rounded = value.toFixed(0);
    if (base === undefined || base === 0) {
        return rounded;
    }
    const percent = Math.round(((value - base) / base) * PERCENT);

    return percent === 0 ? rounded : `${rounded} (${percent > 0 ? "+" : ""}${percent}%)`;
}
