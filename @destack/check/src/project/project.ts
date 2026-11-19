import { createRequire } from "node:module";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { Environment } from "@destack/environment";
import { checkPackage, fixPackage } from "../check/check.ts";
import { configurePackage } from "../check/configuration.ts";
import { formatPackage } from "../check/format.ts";
import { type Manifest, readManifest } from "../check/manifest.ts";
import { runTool, type ToolResult } from "../check/tool.ts";
import type { CheckResult, Diagnostic } from "../diagnostic/diagnostic.ts";
import { CheckError } from "../error/index.ts";
import { Readme } from "../markdown/index.ts";
import { Toolchain } from "../toolchain/index.ts";
import { listChangedModules } from "../workspace/index.ts";

/** The test tiers a run selects, every tier but bench by default. */
export type TestTier = "unit" | "scenario" | "bench";

/** The tiers a test run includes unless asked: what CI and `destack test` check. */
const DEFAULT_TIERS: readonly TestTier[] = ["unit", "scenario"];

/** The most test workers a run starts, two cores each, as scenarios run workloads beside them. */
const MOST_WORKERS = 4;

/** What a test run selects: its tiers, files and database, and whether it only lists the tests. */
export interface TestOptions {
    /** The tiers, unit and scenario by default. */
    readonly tiers?: readonly TestTier[];
    /** The test files, every file by default. */
    readonly files?: readonly string[];
    /** The one database to run against, every planned one by default. */
    readonly database?: "sqlite" | "postgresql";
    /** Whether to list the tests instead of running them. */
    readonly isListing?: boolean;
    /** Whether to write the snapshots the tests compare against anew. */
    readonly isUpdating?: boolean;
}

/** A file the formatter would change, as its check lists it with the time it took. */
const FORMAT_LISTING = /^(\S.*?) \(\d+m?s\)$/u;

/** A plain TypeScript diagnostic line: file, line, column, severity, code and message. */
const TYPESCRIPT_DIAGNOSTIC = /^(.+?)\((\d+),(\d+)\): (error|warning) (TS\d+): (.*)$/u;

/** A package the toolchain checks and fixes, as `destack check` and `destack fix` do. */
export class Project {
    /** The package's directory. */
    readonly directory: string;
    /** The package's manifest. */
    readonly manifest: Manifest;
    /** The tools the project runs. */
    readonly toolchain: Toolchain;
    /** The files, processes and machine the tools run on. */
    readonly environment: Environment<"files" | "process" | "host">;

    /** Work on a package with a toolchain. */
    private constructor(
        directory: string,
        manifest: Manifest,
        toolchain: Toolchain,
        environment: Environment<"files" | "process" | "host">,
    ) {
        // set the package, its manifest, its tools and its environment
        this.directory = directory;
        this.manifest = manifest;
        this.toolchain = toolchain;
        this.environment = environment;
    }

    /** Open the package in a directory, refusing a directory without a package manifest. */
    static async open(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        toolchain: Toolchain = Toolchain.workspace(),
    ): Promise<Project> {
        // read the manifest of the package's real directory, which the tools report paths in
        const absolute = await environment.files.realPath(resolve(directory));
        const manifest = await readManifest(absolute, environment.files);
        if (manifest === undefined) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `package '${absolute}' is invalid: it has no package.json`,
            );
        }

        return new Project(absolute, manifest, toolchain, environment);
    }

    /** Check formatting, types, README listings and lints, then run the package's own check. */
    async check(options: { readonly since?: string } = {}): Promise<CheckResult> {
        // check formatting, then types and the README's listings
        const formatted = Project.#formatDiagnostics(await formatPackage(this.#options(), false));
        const typed = Project.#typescriptDiagnostics(await this.#typecheck(), this.directory);
        const listed = Project.#typescriptDiagnostics(
            await Readme.check(this.directory, this.toolchain, this.environment),
            this.directory,
        );

        // lint every module, or the modules changed since a base, none when none changed
        const files =
            options.since === undefined
                ? undefined
                : await listChangedModules(this.directory, options.since, this.environment);
        const linted =
            files?.length === 0 ? [] : (await checkPackage(this.#options(files))).diagnostics;

        // run the package's own check last
        const own = await this.#script("check");

        return this.#relative([...formatted, ...typed, ...listed, ...linted, ...own]);
    }

    /** Write the generated settings, format and fix lints, then run the package's own fix. */
    async fix(): Promise<CheckResult> {
        // configure, format and fix
        await this.configure();
        const formatted = await formatPackage(this.#options(), true);
        const fixed = await fixPackage(this.#options());

        // run the package's own fix last
        const own = await this.#script("fix");

        return this.#relative([
            ...Project.#formatDiagnostics(formatted),
            ...fixed.diagnostics,
            ...own,
        ]);
    }

    /** Run the package's tests through the plan its files derive, returning the run's exit code. */
    async test(options: TestOptions = {}): Promise<number> {
        // resolve the package's own Vitest and test plan, as its tests import them
        const require = createRequire(join(this.directory, "package.json"));
        const vitest = join(dirname(require.resolve("vitest/package.json")), "vitest.mjs");
        const configuration = require.resolve("@destack/test/plan/vitest");
        const plan: unknown = await import(
            pathToFileURL(require.resolve("@destack/test/plan")).href
        );
        if (!isTestPlanModule(plan)) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `package '${this.directory}' is invalid: its @destack/test exports no TestPlan`,
            );
        }
        const planned = await plan.TestPlan.of(this.directory, this.environment.files);
        const hasNode = planned.targets.some((target) => target.kind === "node");

        // run the Bun targets, then the Node targets apart, as Vitest runs one runtime per process
        const bun = await this.#vitest(
            this.environment.process.executable,
            ["--bun"],
            vitest,
            configuration,
            options,
            "bun",
        );
        const node = hasNode
            ? await this.#vitest("node", [], vitest, configuration, options, "node")
            : 0;

        return bun !== 0 ? bun : node;
    }

    /** Run Vitest over the plan's targets of a runtime, as the options select them. */
    async #vitest(
        executable: string,
        flags: readonly string[],
        vitest: string,
        configuration: string,
        options: TestOptions,
        runtime: "bun" | "node",
    ): Promise<number> {
        // run the targets of the asked tiers and database, on half the cores up to four workers
        const { process } = this.environment;
        const halved = Math.floor(navigator.hardwareConcurrency / 2);
        const workers = Math.max(1, Math.min(MOST_WORKERS, halved));
        const tiers = options.tiers ?? DEFAULT_TIERS;
        const database =
            options.database === undefined ? [] : ["--project", `*:${options.database}`];
        const child = process.spawn(
            [
                executable,
                ...flags,
                vitest,
                // collect a listing by importing each file, as tests register through helpers
                ...(options.isListing === true ? ["list", "--staticParse=false"] : ["run"]),
                "--config",
                configuration,
                "--configLoader",
                "native",
                "--passWithNoTests",
                `--maxWorkers=${workers}`,
                ...(options.isUpdating === true ? ["--update"] : []),
                ...database,
                ...(options.files ?? []),
            ],
            {
                workingDirectory: this.directory,
                variables: {
                    ...process.variables,
                    DESTACK_TEST_TIERS: tiers.join(","),
                    DESTACK_TEST_RUNTIME: runtime,
                },
                stdout: "inherit",
                stderr: "inherit",
            },
        );

        return await child.exited;
    }

    /** Write the settings the toolchain generates for the package. */
    async configure(): Promise<void> {
        await configurePackage(this.directory, this.toolchain, this.environment.files);
    }

    /** Report diagnostics with their files relative to the package, as every tool names them. */
    #relative(diagnostics: readonly Diagnostic[]): CheckResult {
        return {
            diagnostics: diagnostics.map((each) => ({
                ...each,
                filename: isAbsolute(each.filename)
                    ? relative(this.directory, each.filename)
                    : each.filename,
            })),
        };
    }

    /** The options of the toolchain's tools for this package. */
    #options(files?: readonly string[]) {
        return {
            directory: this.directory,
            toolchain: this.toolchain,
            environment: this.environment,
            ...(files === undefined ? {} : { files: [...files] }),
        };
    }

    /** Typecheck the package's sources and tests with its generated settings. */
    #typecheck(): Promise<ToolResult> {
        const executable = join(
            this.toolchain.locate(["typescript"], Toolchain.packageDirectory()),
            "bin",
            "tsc",
        );

        return runTool(
            executable,
            ["--project", "tsconfig.json", "--pretty", "false"],
            this.directory,
            this.environment,
        );
    }

    /** Run the package's own script of a name, none when it has none, failing as one diagnostic. */
    async #script(name: string): Promise<Diagnostic[]> {
        // run nothing for a package without the script
        if (this.manifest.scripts?.[name] === undefined) {
            return [];
        }

        // run it with the package's runtime, reporting a failure with its output
        const { code, stdout, stderr } = await runTool(name, [], this.directory, this.environment);

        return code === 0
            ? []
            : [
                  {
                      code: `script/${name}`,
                      message: `script '${name}' failed with ${code}: ${(stderr || stdout).trim()}`,
                      severity: "error",
                      filename: "package.json",
                      labels: [],
                  },
              ];
    }

    /** Read the formatter's output as one diagnostic per file it would change. */
    static #formatDiagnostics(result: ToolResult): Diagnostic[] {
        // report nothing once every file is formatted
        if (result.code === 0) {
            return [];
        }

        // name each listed file, else the formatter's failure
        const listed = result.stdout.split("\n").flatMap((line) => {
            const [, filename] = FORMAT_LISTING.exec(line.trim()) ?? [];

            return filename === undefined ? [] : [filename];
        });
        const filenames = listed.length === 0 ? ["."] : listed;

        return filenames.map((filename) => ({
            code: "format",
            message:
                listed.length === 0
                    ? `formatting failed: ${result.stderr.trim()}`
                    : "file is not formatted: run destack fix",
            severity: "error",
            filename,
            labels: [],
        }));
    }

    /** Read TypeScript's output as diagnostics, else its failure as one diagnostic. */
    static #typescriptDiagnostics(result: ToolResult, directory: string): Diagnostic[] {
        // report nothing once the program typechecks
        if (result.code === 0) {
            return [];
        }

        // read each diagnostic line, else the compiler's failure
        const read = result.stdout.split("\n").flatMap((line): Diagnostic[] => {
            const [, filename, lineNumber, column, severity, code, message] =
                TYPESCRIPT_DIAGNOSTIC.exec(line.trim()) ?? [];
            if (filename === undefined || message === undefined) {
                return [];
            }

            return [
                {
                    code: code ?? "typescript",
                    message,
                    severity: severity === "warning" ? "warning" : "error",
                    filename,
                    labels: [
                        {
                            span: {
                                offset: 0,
                                length: 0,
                                line: Number(lineNumber),
                                column: Number(column),
                            },
                        },
                    ],
                },
            ];
        });

        return read.length > 0
            ? read
            : [
                  {
                      code: "typescript",
                      message: `typecheck failed in '${directory}': ${(result.stderr || result.stdout).trim()}`,
                      severity: "error",
                      filename: "tsconfig.json",
                      labels: [],
                  },
              ];
    }
}

/** What `destack test` reads of a package's test plan: whether it has Node targets. */
interface TestPlanModule {
    /** The plan of a package's tests. */
    readonly TestPlan: {
        /** Plan the tests of the package in a directory. */
        readonly of: (
            directory: string,
            files: Environment<"files">["files"],
        ) => Promise<{ readonly targets: readonly { readonly kind: string }[] }>;
    };
}

/** Report whether a module is @destack/test's plan module. */
function isTestPlanModule(module: unknown): module is TestPlanModule {
    return typeof module === "object" && module !== null && "TestPlan" in module;
}
