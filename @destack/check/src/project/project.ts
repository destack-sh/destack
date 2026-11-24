import { createRequire } from "node:module";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import type { Environment } from "@destack/environment";
import { checkPackage, fixPackage } from "../check/check.ts";
import { configurePackage } from "../check/configuration.ts";
import { formatPackage } from "../check/format.ts";
import { type Manifest, readManifest } from "../check/manifest.ts";
import { runBun, runTool, type ToolResult } from "../check/tool.ts";
import type { CheckResult, Diagnostic } from "../diagnostic/diagnostic.ts";
import { CheckError } from "../error/index.ts";
import { Readme } from "../markdown/index.ts";
import { Toolchain } from "../toolchain/index.ts";
import { Checkout, listChangedModules, Workspace } from "../workspace/index.ts";
import { Digest, schema } from "@destack/schema";
import type { TestPlan } from "@destack/test/plan";
import { TestStore } from "./store.ts";

/** Vitest's JSON report: each test file's outcome and span per project, in epoch milliseconds. */
const VitestReport = schema.looseObject({
    testResults: schema.array(
        schema.looseObject({
            name: schema.string(),
            status: schema.string(),
            startTime: schema.number(),
            endTime: schema.number(),
        }),
    ),
});

/** The test tiers a run selects, every tier but bench by default. */
export type TestTier = "unit" | "scenario" | "bench";

/** The tiers a test run includes by default. */
const DEFAULT_TIERS: readonly TestTier[] = ["unit", "scenario"];

/** The most test workers a run starts, at two cores each. */
const MOST_WORKERS = 4;

/** What a test run selects. */
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
    /** The pattern the tests' full names match, as Playwright's grep. */
    readonly grep?: string;
}

/** A plain TypeScript diagnostic line. */
const TYPESCRIPT_DIAGNOSTIC = /^(.+?)\((\d+),(\d+)\): (error|warning) (TS\d+): (.*)$/u;

/** A package the toolchain checks, fixes and tests. */
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
        // keep the package, its manifest, tools and environment
        this.directory = directory;
        this.manifest = manifest;
        this.toolchain = toolchain;
        this.environment = environment;
    }

    /** Open the package in a directory, refusing one without a manifest. */
    static async open(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
        toolchain: Toolchain = Toolchain.workspace(),
    ): Promise<Project> {
        // read the manifest of the package's real directory
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
        // check formatting, types and the README
        const started = this.environment.clock.now();
        const formatted = Project.#formatDiagnostics(await formatPackage(this.#options(), false));
        const typed = Project.#typescriptDiagnostics(await this.#typecheck(), this.directory);
        const listed = Project.#typescriptDiagnostics(
            await Readme.check(this.directory, this.toolchain, this.environment),
            this.directory,
        );

        // lint every module or the changed ones
        const files =
            options.since === undefined
                ? undefined
                : await listChangedModules(this.directory, options.since, this.environment);
        const linted =
            files?.length === 0 ? [] : (await checkPackage(this.#options(files))).diagnostics;

        // run the package's own check last
        const own = await this.#script("check");

        // record how long a whole check took in the shared bucket, which schedules CI's runners
        const store = await TestStore.open(this.environment);
        if (options.since === undefined && store.bucket !== undefined) {
            await store.check(await this.member(), this.environment.clock.now() - started);
        }

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

    /** Run the package's tests, skipping files that passed over the same inputs. */
    async test(options: TestOptions = {}): Promise<number> {
        // plan the tests with the test kit and Vitest
        const { TestPlan } = await import("@destack/test/plan");
        const planned = await TestPlan.of(this.directory, this.environment.files);
        const hasNode = planned.targets.some((target) => target.kind === "node");

        // run listings, updates, benches and filtered runs as asked, recording nothing
        const isRecorded =
            options.isListing !== true &&
            options.isUpdating !== true &&
            options.grep === undefined &&
            options.tiers?.includes("bench") !== true;
        if (!isRecorded) {
            return await this.#run(options, hasNode, undefined);
        }

        // select the files that have not passed over the package's closure
        const store = await TestStore.open(this.environment);
        const member = await this.member();
        const closure = await this.closure(options);
        const passed = await store.passed(this.directory, member, closure);
        const named = options.files ?? [];
        const selected =
            named.length === 0
                ? this.files(planned, options)
                : named.map((file) => this.#file(file));
        const missing = selected.filter((file) => !passed.has(file));
        if (missing.length === 0) {
            this.environment.process.stderr.writeSync(
                new TextEncoder().encode(
                    `tests passed before over the same inputs (${closure.slice(0, 12)})\n`,
                ),
            );

            return 0;
        }

        // run the missing files, then record the ones that passed
        const directory = await this.environment.files.makeTemporaryDirectory("destack-test-");
        try {
            const code = await this.#run({ ...options, files: missing }, hasNode, directory);
            const durations = await this.#passes(directory);
            await store.pass(this.directory, member, closure, durations);

            return code;
        } finally {
            await this.environment.files.remove(directory, { recursive: true, force: true });
        }
    }

    /** List the planned test files in the selected tiers and database. */
    files(planned: TestPlan, options: TestOptions): string[] {
        // keep the targets in the selected tiers and database
        const tiers = options.tiers ?? DEFAULT_TIERS;
        const targets = planned.targets.filter(
            (target) =>
                tiers.includes(target.tier) &&
                (options.database === undefined || target.database === options.database),
        );

        return [...new Set(targets.flatMap((target) => target.files))].toSorted();
    }

    /** Read the package's directory relative to its workspace root, its name in the test store. */
    async member(): Promise<string> {
        const workspace = await this.#workspace();

        return relative(workspace.root, this.directory) || ".";
    }

    /** Digest what a test run reads besides its files: packages, lockfile, runtime, options. */
    async closure(options: TestOptions): Promise<Digest> {
        // list the package and its dependencies
        const { files, process, crypto } = this.environment;
        const workspace = await this.#workspace();
        const directories = workspace
            .dependencies(await this.member())
            .map((each) => join(workspace.root, each));

        // key each package by its checkout's blobs, and the lockfile by its bytes
        const checkout = await Checkout.list(workspace.root, this.environment);
        const packages = await Promise.all(
            directories
                .toSorted((left, right) => left.localeCompare(right))
                .map(async (each) => [
                    relative(workspace.root, each),
                    await checkout.identify(each),
                ]),
        );
        const lockfile = await Digest.of(
            await files.readFile(join(workspace.root, "bun.lock")),
            crypto,
        );
        const digests = [...packages, ["bun.lock", lockfile]];

        // key the run by its tiers, database and test variables, never by its files
        const selection = { tiers: options.tiers ?? DEFAULT_TIERS, database: options.database };
        const variables = Object.entries(process.variables)
            .filter(([name]) => name.startsWith("DESTACK_TEST_"))
            .toSorted(([left], [right]) => left.localeCompare(right));

        return await Digest.json(
            { digests, runtime: process.runtime.revision, selection, variables },
            crypto,
        );
    }

    /** Run the Bun targets, then the Node targets, writing results into a directory if given. */
    async #run(
        options: TestOptions,
        hasNode: boolean,
        results: string | undefined,
    ): Promise<number> {
        // find Vitest and the plan's configuration
        const require = createRequire(import.meta.url);
        const vitest = join(dirname(require.resolve("vitest/package.json")), "vitest.mjs");
        const configuration = require.resolve("@destack/test/plan/vitest");

        // run Bun's targets, then Node's
        const bun = await this.#vitest(
            this.environment.process.executable,
            ["--bun"],
            vitest,
            configuration,
            options,
            "bun",
            results === undefined ? undefined : join(results, "bun.json"),
        );
        const node = hasNode
            ? await this.#vitest(
                  "node",
                  [],
                  vitest,
                  configuration,
                  options,
                  "node",
                  results === undefined ? undefined : join(results, "node.json"),
              )
            : 0;

        return bun !== 0 ? bun : node;
    }

    /** Read the files that passed in every project and runtime, with their summed durations. */
    async #passes(directory: string): Promise<Map<string, number>> {
        // sum each file's durations and note any failure, over both runtimes' results
        const { files } = this.environment;
        const durations = new Map<string, number>();
        const failed = new Set<string>();
        for (const name of ["bun.json", "node.json"]) {
            const path = join(directory, name);
            if (await files.exists(path)) {
                const report = VitestReport.parse(JSON.parse(await files.readTextFile(path)));
                for (const result of report.testResults) {
                    const file = relative(this.directory, result.name);
                    const milliseconds = result.endTime - result.startTime;
                    durations.set(file, (durations.get(file) ?? 0) + milliseconds);
                    if (result.status !== "passed") {
                        failed.add(file);
                    }
                }
            }
        }

        // keep the files without a failure
        for (const file of failed) {
            durations.delete(file);
        }

        return durations;
    }

    /** Read a file argument relative to the package, as the plan lists its files. */
    #file(file: string): string {
        return isAbsolute(file) ? relative(this.directory, file) : file;
    }

    /** Read the workspace the package belongs to, refusing a package in none. */
    async #workspace(): Promise<Workspace> {
        const workspace = await Workspace.read(this.directory, this.environment.files);
        if (workspace === undefined) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `package '${this.directory}' is invalid: it is in no workspace`,
            );
        }

        return workspace;
    }

    /** Run Vitest over a runtime's selected targets. */
    async #vitest(
        executable: string,
        flags: readonly string[],
        vitest: string,
        configuration: string,
        options: TestOptions,
        runtime: "bun" | "node",
        results: string | undefined,
    ): Promise<number> {
        // run the selected targets on up to four workers, naming the results file the configuration writes
        const { process } = this.environment;
        const { RESULTS_VARIABLE } = await import("@destack/test/plan");
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
                // list the tests by importing each file
                ...(options.isListing === true ? ["list", "--staticParse=false"] : ["run"]),
                "--config",
                configuration,
                "--configLoader",
                "native",
                "--passWithNoTests",
                `--maxWorkers=${workers}`,
                ...(options.isUpdating === true ? ["--update"] : []),
                ...(options.grep === undefined ? [] : ["--testNamePattern", options.grep]),
                ...database,
                ...(options.files ?? []),
            ],
            {
                workingDirectory: this.directory,
                variables: {
                    ...process.variables,
                    DESTACK_TEST_TIERS: tiers.join(","),
                    DESTACK_TEST_RUNTIME: runtime,
                    ...(results === undefined ? {} : { [RESULTS_VARIABLE]: results }),
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

    /** Report diagnostics with paths relative to the package. */
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
        return runBun(
            ["check", "--no-script", "--project", "tsconfig.json", "--no-pretty"],
            "bun check",
            this.directory,
            this.environment,
        );
    }

    /** Run the package's own script, failing as one diagnostic. */
    async #script(name: string): Promise<Diagnostic[]> {
        // skip a missing script
        if (this.manifest.scripts?.[name] === undefined) {
            return [];
        }

        // run it, reporting a failure with its output
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

    /** Read the formatter's output as one diagnostic per changed file. */
    static #formatDiagnostics(result: ToolResult): Diagnostic[] {
        // report nothing for formatted files
        if (result.code === 0) {
            return [];
        }

        // read the unformatted files
        const listed = result.stdout
            .split("\n")
            .map((line) => line.trim())
            .filter((line) => line !== "");
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

    /** Read TypeScript's output as diagnostics. */
    static #typescriptDiagnostics(result: ToolResult, directory: string): Diagnostic[] {
        // report nothing for a clean program
        if (result.code === 0) {
            return [];
        }

        // read each diagnostic line
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
