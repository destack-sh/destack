import { createRequire } from "node:module";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import type { ViteUserConfig } from "vitest/config";
import type { Environment, FileSystem } from "@destack/environment";
import { PackageDefinition, type TestDatabase } from "@destack/package";
import type { TransformContext } from "@destack/package/build";
import type { PluginOption } from "vite";
import { present, schema } from "@destack/schema";
import {
    defineConfiguration,
    type ProjectConfiguration,
    scenarioPlugin,
    type TestTier,
} from "../config/config.ts";

/** The directories a package's tests live in. */
const TEST_DIRECTORIES = ["src", "tests"] as const;

/** The directories whose files are never tests: installed packages and fixtures. */
const IGNORED = new Set(["node_modules", "fixture", "fixtures"]);

/** The global setup a package's tests run once, when the package has one. */
const SETUP = "src/test/setup.ts";

/** A package's test kit, whose drivers play its scenario modules. */
const TEST_KIT = "src/test/index.ts";

/** The builds and workers a machine places, which Bun loads itself rather than the test server. */
const MACHINE_FILES = /\/destack-machine-[^/]+\/(?:builds|workers)\//u;

/** The package.json fields a plan reads: the development dependencies its tests declare. */
const PackageManifest = schema.looseObject({
    devDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
});

/** The databases a package's tests run against when its definition names none. */
const DEFAULT_DATABASES = ["sqlite"] as const;

/** Where a test runs. */
export type TestKind = "bun" | "node" | "dom" | "browser" | "server" | "hydration" | "typecheck";

/** The runtime a test run starts: Node for the Node targets, Bun for every other. */
export type TestRuntime = "bun" | "node";

/** One runnable set of a package's test files, in one tier, kind and database. */
export interface TestTarget {
    /** The target's name, such as `unit:dom:sqlite`, which reports and filters use. */
    readonly name: string;
    /** How the tests cost: unit, scenario or bench. */
    readonly tier: TestTier;
    /** Where the tests run. */
    readonly kind: TestKind;
    /** The database the tests run against, absent for types alone. */
    readonly database: TestDatabase | undefined;
    /** The test files, relative to the package. */
    readonly files: readonly string[];
    /** The global setup, relative to the package, absent for a package without one. */
    readonly setup: string | undefined;
}

/** A test file's suffix: its tier and kind, and whether bench plays it too. */
interface TestSuffix {
    /** The suffix that marks it. */
    readonly suffix: RegExp;
    /** How it costs. */
    readonly tier: TestTier;
    /** Where it runs. */
    readonly kind: TestKind;
    /** Whether the bench tier measures it too. */
    readonly isBenched: boolean;
}

/** The test files' suffixes, most specific first. */
const SUFFIXES: readonly TestSuffix[] = [
    { suffix: /\.test-d\.ts$/u, tier: "unit", kind: "typecheck", isBenched: false },
    {
        suffix: /\.scenario\.test\.tsx$/u,
        tier: "scenario",
        kind: "dom",
        isBenched: false,
    },
    {
        suffix: /\.scenario\.test\.ts$/u,
        tier: "scenario",
        kind: "bun",
        isBenched: false,
    },
    {
        suffix: /\.browser\.scenario\.tsx$/u,
        tier: "scenario",
        kind: "browser",
        isBenched: false,
    },
    { suffix: /\.scenario\.tsx$/u, tier: "scenario", kind: "dom", isBenched: true },
    { suffix: /\.scenario\.ts$/u, tier: "scenario", kind: "bun", isBenched: true },
    { suffix: /\.node\.test\.ts$/u, tier: "unit", kind: "node", isBenched: false },
    {
        suffix: /\.server\.test\.tsx?$/u,
        tier: "unit",
        kind: "server",
        isBenched: false,
    },
    {
        suffix: /\.hydration\.test\.tsx$/u,
        tier: "unit",
        kind: "hydration",
        isBenched: false,
    },
    { suffix: /\.test\.tsx$/u, tier: "unit", kind: "dom", isBenched: false },
    { suffix: /\.test\.ts$/u, tier: "unit", kind: "bun", isBenched: false },
];

/** Every target of a package's tests, derived from its files and its definition. */
export class TestPlan {
    /** The package's directory. */
    readonly directory: string;
    /** The targets, in a stable order. */
    readonly targets: readonly TestTarget[];
    /** Whether the package runs in Bun alone, so tests resolve with Bun's conditions. */
    readonly isBunOnly: boolean;

    /** Plan a package's targets. */
    private constructor(directory: string, targets: readonly TestTarget[], isBunOnly: boolean) {
        // set the package, its targets and its runtimes
        this.directory = directory;
        this.targets = targets;
        this.isBunOnly = isBunOnly;
    }

    /** Plan the tests of the package in a directory from its files and its definition. */
    static async of(directory: string, files: FileSystem): Promise<TestPlan> {
        // read the databases the definition names, SQLite alone by default
        const definition = PackageDefinition.parse(
            JSON.parse(await files.readTextFile(join(directory, "destack.json"))),
        );
        const databases = definition.test?.databases ?? DEFAULT_DATABASES;

        // list the test files and the setup
        const listed = await TestPlan.#list(directory, files);
        const setup = (await files.exists(join(directory, SETUP))) ? SETUP : undefined;

        // read whether the package runs in Bun alone, outside any browser
        const isBunOnly = definition.runtimes?.includes("browser") !== true;

        return new TestPlan(directory, TestPlan.derive(listed, databases, setup), isBunOnly);
    }

    /** Derive a package's targets: one per tier, kind and database that has files. */
    static derive(
        listed: readonly string[],
        databases: readonly TestDatabase[],
        setup: string | undefined,
    ): TestTarget[] {
        // sort each file into its tier and environment, a scenario module into bench too
        const sorted = [...listed].toSorted().flatMap((file) => {
            // find the file's kind, leaving files of no kind out
            const suffix = SUFFIXES.find((each) => each.suffix.test(file));
            if (suffix === undefined) {
                return [];
            }
            const placed = { tier: suffix.tier, kind: suffix.kind, file };

            return suffix.isBenched ? [placed, { ...placed, tier: "bench" as const }] : [placed];
        });
        const groups = Map.groupBy(sorted, (each) => `${each.tier}:${each.kind}`);

        // run each group per database, types once without one
        return [...groups.values()].flatMap((group): TestTarget[] => {
            // take the group's tier, kind and files
            const { tier, kind } = present(group[0], "a group's first file");
            const files = group.map((each) => each.file);
            const target = { tier, kind, files, setup };

            return kind === "typecheck"
                ? [{ ...target, name: `${tier}:${kind}`, database: undefined }]
                : databases.map((database) => ({
                      ...target,
                      name: `${tier}:${kind}:${database}`,
                      database,
                  }));
        });
    }

    /** Configure Vitest with one project per target of a runtime, as `destack test` runs it. */
    async configuration(
        environment: Environment<"files" | "process">,
        runtime: TestRuntime = "bun",
    ): Promise<ViteUserConfig> {
        // play scenario modules through the test kit's drivers, refusing a package without one
        const hasModules = this.targets.some(
            (target) => target.tier === "bench" && target.kind === "bun",
        );
        const kit = join(this.directory, TEST_KIT);
        if (hasModules && !(await environment.files.exists(kit))) {
            throw new TypeError(
                `package '${this.directory}' plays scenario modules without drivers: export them from ${TEST_KIT}`,
            );
        }

        // transform Bun's modules as a build does, for a package whose tests declare @destack/build
        const manifest = PackageManifest.parse(
            JSON.parse(await environment.files.readTextFile(join(this.directory, "package.json"))),
        );
        const build =
            manifest.devDependencies?.["@destack/build"] === undefined
                ? undefined
                : await this.#kit("@destack/build/local", isBuildKit);
        const context = {
            directory: this.directory,
            runtime: "bun",
            server: true,
            environment,
        } as const;
        const transforms = build === undefined ? [] : await build.loadTransforms(context);

        // configure each target, resolving a package that runs in Bun alone as Bun does
        const targets = this.targets.filter(
            (target) => (target.kind === "node") === (runtime === "node"),
        );
        const projects = await Promise.all(
            targets.map((target) => this.#project(target, environment)),
        );
        const conditions = { conditions: ["bun"], externalConditions: ["bun"] };

        return defineConfiguration(
            {
                root: this.directory,
                ...(this.isBunOnly ? { ssr: { resolve: conditions } } : {}),
                plugins: [...transforms, ...(hasModules ? [scenarioPlugin([kit])] : [])],
                test: { projects, environment: "node" },
            },
            environment,
        );
    }

    /** Configure one target: Bun, Node and types extend the plan, renders use the view kit. */
    async #project(
        target: TestTarget,
        environment: Environment<"files" | "process">,
    ): Promise<ProjectConfiguration> {
        // name the target's tests, which its database names further, with the setup
        const { tier, kind: runs, database, files, setup } = target;
        const base = {
            tier,
            ...(database === undefined ? {} : { targets: { database: [database] as const } }),
        } as const;
        const test = {
            name: `${tier}:${runs}`,
            ...(setup === undefined ? {} : { globalSetup: [setup] }),
        };

        // run outside a DOM unless rendering, so no transform's plugin chooses an environment instead
        const outside = { ...test, environment: "node" };

        // run Bun tests leaving the machine's builds to Bun, Node tests as they are, and types alone
        if (runs === "node") {
            return { extends: true, ...base, test: { ...outside, include: [...files] } };
        } else if (runs === "bun") {
            const server = { deps: { external: [MACHINE_FILES] } };

            return { extends: true, ...base, test: { ...outside, include: [...files], server } };
        } else if (runs === "typecheck") {
            const typecheck = { enabled: true, only: true, include: [...files] };

            return { extends: true, ...base, test: { ...outside, typecheck } };
        }

        // render through @destack/view, in its DOM, as a server, or hydrating
        const view = await this.#kit("@destack/view/test", isViewTestKit);
        const rendered = view.defineConfiguration(
            {
                tier,
                root: this.directory,
                test: {
                    ...test,
                    include: [...files],
                    ...(runs === "server" ? { environment: "node" } : {}),
                },
            },
            environment,
            { isHydrated: runs === "hydration", isBrowser: runs === "browser" },
        );

        return { extends: false, ...base, ...rendered };
    }

    /** Open a kit of the package's own dependencies, refusing one lacking what the plan calls. */
    async #kit<Kit>(specifier: string, isKit: (module: unknown) => module is Kit): Promise<Kit> {
        // resolve the package's own copy, as its tests import it
        const require = createRequire(join(this.directory, "package.json"));
        const kit: unknown = await import(pathToFileURL(require.resolve(specifier)).href);
        if (!isKit(kit)) {
            throw new TypeError(
                `package '${this.directory}' resolves '${specifier}' without what its tests run through`,
            );
        }

        return kit;
    }

    /** List the files of a package's test directories relative to it, leaving fixtures out. */
    static async #list(directory: string, files: FileSystem): Promise<string[]> {
        // walk each test directory the package has
        const listed: string[] = [];
        for (const name of TEST_DIRECTORIES) {
            if (await files.exists(join(directory, name))) {
                await TestPlan.#walk(directory, name, files, listed);
            }
        }

        return listed;
    }

    /** Add the files below a package directory to a list, skipping ignored directories. */
    static async #walk(
        directory: string,
        path: string,
        files: FileSystem,
        listed: string[],
    ): Promise<void> {
        for (const entry of await files.readDirectory(join(directory, path))) {
            // descend into a directory that can hold tests, and add each file
            const child = join(path, entry.name);
            if (entry.kind === "directory" && !IGNORED.has(entry.name)) {
                await TestPlan.#walk(directory, child, files, listed);
            } else if (entry.kind === "file") {
                listed.push(child);
            }
        }
    }
}

/** What a plan renders tests through: @destack/view's test kit, which depends on this package. */
interface ViewTestKit {
    /** Configure a rendering target's tests. */
    readonly defineConfiguration: (
        configuration: Parameters<typeof defineConfiguration>[0] & { readonly tier: TestTier },
        environment: Environment<"files" | "process">,
        options: { readonly isHydrated: boolean; readonly isBrowser: boolean },
    ) => ViteUserConfig;
}

/** Report whether a module is @destack/view's test kit. */
function isViewTestKit(module: unknown): module is ViewTestKit {
    return (
        typeof module === "object" &&
        module !== null &&
        "defineConfiguration" in module &&
        typeof module.defineConfiguration === "function"
    );
}

/** What a plan transforms Bun's modules with: @destack/build's loader. */
interface BuildKit {
    /** Load the transforms of a package's own build extension and its dependencies'. */
    readonly loadTransforms: (
        context: Omit<TransformContext, "dependencies">,
    ) => Promise<PluginOption[]>;
}

/** Report whether a module is @destack/build's local kit. */
function isBuildKit(module: unknown): module is BuildKit {
    return (
        typeof module === "object" &&
        module !== null &&
        "loadTransforms" in module &&
        typeof module.loadTransforms === "function"
    );
}
