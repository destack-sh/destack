import { createRequire } from "node:module";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import type { Environment } from "@destack/environment";
import { checkPackage, fixPackage, listFiles } from "../check/check.ts";
import { configurePackage } from "../check/configuration.ts";
import { TransformCache } from "@destack/package/build";
import { formatPackage } from "../check/format.ts";
import { type Manifest, readManifest } from "../check/manifest.ts";
import { runBun, runTool, type ToolResult } from "../check/tool.ts";
import type { CheckResult, Diagnostic } from "../diagnostic/diagnostic.ts";
import { CheckError } from "../error/index.ts";
import { Readme } from "../markdown/index.ts";
import { Toolchain } from "../toolchain/index.ts";
import { listChangedModules, Workspace } from "../workspace/index.ts";
import { Digest } from "@destack/schema";

/** The directory of a package's passed test runs, by digest. */
const TEST_CACHE = "node_modules/.cache/destack/test";

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

    /** Run the package's tests through its plan, returning the exit code. */
    async test(options: TestOptions = {}): Promise<number> {
        // reuse a pass over the same inputs, never for listings, updates or benches
        const cache = TransformCache.directory(join(this.directory, TEST_CACHE), this.environment);
        const isKept =
            options.isListing !== true &&
            options.isUpdating !== true &&
            options.tiers?.includes("bench") !== true;
        const key = isKept ? await this.#testKey(options) : undefined;
        if (key !== undefined && (await cache.read(key)) !== undefined) {
            this.environment.process.stderr.writeSync(
                new TextEncoder().encode(
                    `tests passed before over the same inputs (${key.slice(0, 12)})\n`,
                ),
            );

            return 0;
        }

        // plan the tests with the test kit and Vitest
        const { TestPlan } = await import("@destack/test/plan");
        const require = createRequire(import.meta.url);
        const vitest = join(dirname(require.resolve("vitest/package.json")), "vitest.mjs");
        const configuration = require.resolve("@destack/test/plan/vitest");
        const planned = await TestPlan.of(this.directory, this.environment.files);
        const hasNode = planned.targets.some((target) => target.kind === "node");

        // run the Bun targets, then the Node targets
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

        // keep a pass
        const code = bun !== 0 ? bun : node;
        if (code === 0 && key !== undefined) {
            await cache.write(key, new Uint8Array());
        }

        return code;
    }

    /** Digest what a test run reads: its packages' files, the lockfile and the options. */
    async #testKey(options: TestOptions): Promise<Digest> {
        // list the files of the package and its dependencies
        const { files, process, crypto } = this.environment;
        const workspace = await Workspace.read(this.directory, files);
        if (workspace === undefined) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `package '${this.directory}' is invalid: it is in no workspace`,
            );
        }
        const member = relative(workspace.root, this.directory) || ".";
        const directories = workspace
            .dependencies(member)
            .map((each) => join(workspace.root, each));
        const listed = (
            await Promise.all(directories.map((each) => listFiles(each, files)))
        ).flat();
        const paths = [...listed, join(workspace.root, "bun.lock")].toSorted((left, right) =>
            left.localeCompare(right),
        );

        // digest the files and the run's inputs
        const digests = await Promise.all(
            paths.map(async (path) => [
                relative(workspace.root, path),
                await Digest.of(await files.readFile(path), crypto),
            ]),
        );
        const variables = Object.entries(process.variables)
            .filter(([name]) => name.startsWith("DESTACK_TEST_"))
            .toSorted(([left], [right]) => left.localeCompare(right));

        return await Digest.json(
            { digests, runtime: process.runtime.revision, options, variables },
            crypto,
        );
    }

    /** Run Vitest over a runtime's selected targets. */
    async #vitest(
        executable: string,
        flags: readonly string[],
        vitest: string,
        configuration: string,
        options: TestOptions,
        runtime: "bun" | "node",
    ): Promise<number> {
        // run the selected targets on up to four workers
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
