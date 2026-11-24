import { join } from "node:path";
import { Project } from "./project.ts";
import type { Environment } from "@destack/environment";
import type { TestStore } from "./store.ts";

/** The work one CI runner takes, in milliseconds: two minutes, the wall time CI aims for. */
const RUNNER_MILLISECONDS = 120_000;

/** The most runners a schedule uses, below GitHub's limit on concurrent jobs. */
const MOST_RUNNERS = 40;

/** The infix of test files that run in a browser, as the test plan names them. */
const BROWSER_SUFFIX = ".browser.scenario.";

/** The packages read at once, each listing its closure and spawning a git listing. */
const PACKAGE_CONCURRENCY = 8;

/** One package's share of a runner: its check and its test files. */
export interface Work {
    /** The package's directory relative to the workspace root, such as `@destack/object`. */
    readonly package: string;
    /** The test files to run, relative to the package; none runs every file not yet passed. */
    readonly files: readonly string[];
    /** Whether this runner checks the package, which one runner per package does. */
    readonly isChecked: boolean;
}

/** One CI runner's work and its expected duration. */
export interface Runner {
    /** The runner's number, from 1. */
    readonly runner: number;
    /** How long the work should take, in milliseconds. */
    readonly milliseconds: number;
    /** Whether any of its test files runs in a browser, so the runner installs Chromium. */
    readonly isBrowsing: boolean;
    /** The packages and files it checks and tests. */
    readonly work: readonly Work[];
}

/** A package's work before packing: its missing files, weighted by their last durations. */
interface Item {
    /** The package's directory relative to the workspace root. */
    readonly package: string;
    /** The files, with their weights. */
    readonly files: readonly (readonly [file: string, milliseconds: number])[];
    /** The weight of the files, with the package's check when this item checks it. */
    readonly weight: number;
    /** Whether this item's runner checks the package. */
    readonly isChecked: boolean;
    /** Whether this item is the whole package, run without naming its files. */
    readonly isWhole: boolean;
}

/** The CI runners for packages: their missing test files, packed by their last durations. */
export class Schedule {
    /** The runners, each with its work. */
    readonly runners: readonly Runner[];

    /** Keep the packed runners. */
    constructor(runners: readonly Runner[]) {
        this.runners = runners;
    }

    /** Schedule the checks of packages and the test files not yet passed over their closures. */
    static async of(
        root: string,
        packages: readonly string[],
        store: TestStore,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<Schedule> {
        // read each package's missing files and last durations, a few packages at once
        const { TestPlan } = await import("@destack/test/plan");
        const read = await eachLimited(packages, PACKAGE_CONCURRENCY, async (name) => {
            // list the files not yet passed over the package's closure
            const project = await Project.open(join(root, name), environment);
            const planned = await TestPlan.of(project.directory, environment.files);
            const closure = await project.closure({});
            const passed = await store.passed(project.directory, name, closure);
            const missing = project.files(planned, {}).filter((file) => !passed.has(file));

            // read the package's last durations
            const durations = await store.durations(name);
            const check = await store.checked(name);

            return { name, missing, durations, check };
        });

        // weigh unknown files and checks by every package's median
        const fileMedian = median(read.flatMap(({ durations }) => [...durations.values()]));
        const checkMedian = median(
            read.flatMap(({ check }) => (check === undefined ? [] : [check])),
        );
        if (fileMedian === undefined || checkMedian === undefined) {
            throw new Error(
                "the test store has no durations to schedule by: record a full run first",
            );
        }

        // weigh each package's missing files and its check
        const items = read.map(({ name, missing, durations, check }) => {
            // weigh the files by their own, their package's or every package's durations
            const own = median([...durations.values()]);
            const files = missing.map(
                (file) => [file, durations.get(file) ?? own ?? fileMedian] as const,
            );

            return {
                package: name,
                files,
                weight: sum(files.map(([, value]) => value)),
                check: check ?? checkMedian,
            };
        });

        // split packages heavier than an average runner by their files, the first part checking the package
        const total = sum(items.map((item) => item.weight + item.check));
        const count = Math.min(MOST_RUNNERS, Math.max(1, Math.ceil(total / RUNNER_MILLISECONDS)));
        const average = total / count;
        const parts = items.flatMap((item): Item[] => {
            // split a heavy package's files into parts of about an average runner
            if (item.weight > average && item.files.length > 1) {
                const bins = pack(
                    item.files,
                    Math.ceil(item.weight / average),
                    ([, value]) => value,
                );

                return bins
                    .filter((bin) => bin.items.length > 0)
                    .map((bin, index) => ({
                        package: item.package,
                        files: bin.items,
                        weight: bin.weight + (index === 0 ? item.check : 0),
                        isChecked: index === 0,
                        isWhole: false,
                    }));
            }
            // keep a light package whole
            else {
                return [
                    {
                        package: item.package,
                        files: item.files,
                        weight: item.weight + item.check,
                        isChecked: true,
                        isWhole: true,
                    },
                ];
            }
        });

        // pack the parts onto the runners
        const bins = pack(parts, count, (part) => part.weight).filter(
            (bin) => bin.items.length > 0,
        );

        // describe each runner's work, grouped by package
        const runners = bins.map((bin, index): Runner => {
            // group the runner's parts by package
            const byPackage = Map.groupBy(bin.items, (part) => part.package);
            const work = [...byPackage].map(([name, group]): Work => {
                // run a whole package's missing files, or the named ones of a split package
                const isWhole = group.some((part) => part.isWhole);
                const files = isWhole
                    ? []
                    : group.flatMap((part) => part.files.map(([file]) => file));

                return {
                    package: name,
                    files: files.toSorted(),
                    isChecked: group.some((part) => part.isChecked),
                };
            });

            // note whether a file it runs needs a browser
            const isBrowsing = bin.items.some((part) =>
                part.files.some(([file]) => file.includes(BROWSER_SUFFIX)),
            );

            return { runner: index + 1, milliseconds: Math.round(bin.weight), isBrowsing, work };
        });

        return new Schedule(runners);
    }
}

/** Pack items onto a number of bins, heaviest first onto the lightest bin. */
function pack<Packed>(
    items: readonly Packed[],
    count: number,
    weigh: (item: Packed) => number,
): { items: Packed[]; weight: number }[] {
    // start with empty bins
    const bins = Array.from({ length: count }, (): { items: Packed[]; weight: number } => ({
        items: [],
        weight: 0,
    }));

    // place each item, heaviest first, on the lightest bin
    for (const item of items.toSorted((left, right) => weigh(right) - weigh(left))) {
        const lightest = bins.reduce((least, bin) => (bin.weight < least.weight ? bin : least));
        lightest.items.push(item);
        lightest.weight += weigh(item);
    }

    return bins;
}

/** Read the median of values, absent for none. */
function median(values: readonly number[]): number | undefined {
    const sorted = values.toSorted((left, right) => left - right);

    return sorted[Math.floor(sorted.length / 2)];
}

/** Add values. */
function sum(values: readonly number[]): number {
    return values.reduce((total, value) => total + value, 0);
}

/** Map items through an action, at most a number at once, keeping their order. */
async function eachLimited<Input, Result>(
    items: readonly Input[],
    limit: number,
    action: (item: Input) => Promise<Result>,
): Promise<Result[]> {
    // take the next item until none remain, in each of the concurrent loops
    const results: Result[] = [];
    const queue = [...items.entries()].toReversed();
    const work = async () => {
        for (let entry = queue.pop(); entry !== undefined; entry = queue.pop()) {
            const [index, item] = entry;
            results[index] = await action(item);
        }
    };
    await Promise.all(Array.from({ length: Math.min(limit, items.length) }, work));

    return results;
}
