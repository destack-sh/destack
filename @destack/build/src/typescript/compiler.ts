import { join, resolve } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import { fileURLToPath } from "node:url";
import { API, type Diagnostic, type Project } from "typescript/unstable/async";
import { Toolchain } from "@destack/check/toolchain";
import { BuildError } from "../error/index.ts";
import { TypeScriptInspection } from "./module.ts";
import { isAuthored, linkDependencies } from "../source/dependency.ts";
import { runtimeConditions, TYPE_CHECKS, TYPESCRIPT_OPTIONS } from "@destack/package/build";
import type { PackageSource } from "../source/index.ts";

/** This package's directory, whose dependencies include the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** Retain TypeScript compiler state across inspections of one package. */
export class TypeScriptCompiler implements AsyncDisposable {
    /** The source package directory. */
    readonly directory: string;
    /** The compiler process and retained project state. */
    readonly #api: API;
    /** The files the configurations resolve through. */
    readonly #files: FileSystem;
    /** The directory of the program configurations, linking the installed dependencies. */
    readonly #configurations: string;

    /** Run the native compiler for a source package's real directory. */
    private constructor(directory: string, api: API, files: FileSystem, configurations: string) {
        // store the package, the compiler process and the configurations' directory
        this.directory = directory;
        this.#api = api;
        this.#files = files;
        this.#configurations = configurations;
    }

    /** Start the native compiler for a source package. */
    static async start(
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<TypeScriptCompiler> {
        // resolve explicit type packages from the same installation as authored modules
        const { files } = environment;
        const real = await files.realPath(directory);
        const configurations = await files.makeTemporaryDirectory("destack-typescript-");
        await linkDependencies(real, configurations, files);

        // start the compiler process
        const api = new API({
            cwd: real,
            tsserverPath: TypeScriptCompiler.executable(environment),
        });

        return new TypeScriptCompiler(real, api, files, configurations);
    }

    /** Find this platform's native TypeScript compiler among the tools. */
    static executable(environment: Environment<"process" | "host">): string {
        // find the platform's native package beside TypeScript
        const { os, architecture } = environment.host;
        const native = `@typescript/typescript-${os}-${architecture}`;
        const directory = Toolchain.locate(["typescript", native], PACKAGE, environment.process);

        return join(directory, "lib", "tsc");
    }

    /** Check a package's whole program, then read its modules and every symbol they lead to. */
    async inspect(source: PackageSource): Promise<TypeScriptInspection> {
        return await this.#read(await this.#configure(source), async (project) => {
            // check the configuration and the authored modules
            const diagnostics = await this.#check(project);
            if (diagnostics.length) {
                throw BuildError.of(
                    "INSPECTION_FAILED",
                    `the TypeScript inspection failed: ${JSON.stringify(diagnostics)}`,
                );
            }

            return await TypeScriptInspection.read(project, source);
        });
    }

    /** Write a source's program configuration at one stable path per runtime. */
    async #configure(source: PackageSource): Promise<string> {
        const path = join(this.#configurations, `${source.runtime}.json`);
        await this.#files.writeTextFile(path, JSON.stringify(programSettings(source)));

        return path;
    }

    /** Collect a program's configuration diagnostics and its authored modules' diagnostics. */
    async #check(project: Project): Promise<Diagnostic[]> {
        const authored = (await project.program.getSourceFileNames()).filter((file) =>
            isAuthored(this.directory, file),
        );

        return (
            await Promise.all([
                project.program.getConfigFileParsingDiagnostics(),
                project.program.getProgramDiagnostics(),
                ...authored.flatMap((file) => [
                    project.program.getSyntacticDiagnostics(file),
                    project.program.getBindDiagnostics(file),
                    project.program.getSemanticDiagnostics(file),
                ]),
            ])
        ).flat();
    }

    /** Read a configuration's project from one fresh snapshot, then release the snapshot. */
    async #read<Result>(
        configuration: string,
        read: (project: Project) => Promise<Result>,
    ): Promise<Result> {
        // refresh the configuration
        const path = await this.#files.realPath(configuration);

        // refresh client ASTs while the native compiler retains incremental project state
        this.#api.clearSourceFileCache();
        const invalidated = await this.#api.updateSnapshot({
            fileChanges: { invalidateAll: true },
        });
        await invalidated.dispose();

        // reread configured include patterns so added and removed modules change the lists
        const snapshot = await this.#api.updateSnapshot({
            openProjects: [path],
            fileChanges: { changed: [path] },
        });

        // read the compiler project and release the snapshot
        try {
            const project = snapshot.getProject(path);
            if (!project) {
                throw BuildError.of(
                    "INSPECTION_FAILED",
                    `the TypeScript compiler did not load ${path}`,
                );
            }

            return await read(project);
        } finally {
            await snapshot.dispose();
        }
    }

    /** Close the compiler and release retained projects. */
    async [Symbol.asyncDispose](): Promise<void> {
        try {
            await this.#api.close();
        } finally {
            await this.#files.remove(this.#configurations, { recursive: true });
        }
    }
}

/** Extend the authored or default configuration with the runtime, checks and dependency builds. */
function programSettings(source: PackageSource): object {
    // resolve the runtime's conditions and the dependencies' builds from the root modules
    const conditions = runtimeConditions(source.runtime);
    const paths = source.tree.paths();
    const files = source.entries;

    // extend the authored configuration with the standard type checks
    if (source.configuration !== undefined) {
        return {
            extends: source.configuration,
            compilerOptions: { ...TYPE_CHECKS, customConditions: conditions, paths },
            files,
        };
    }
    // check the package's sources and tests with the default options
    else {
        return {
            compilerOptions: {
                ...TYPESCRIPT_OPTIONS,
                jsx: "preserve",
                jsxImportSource: "@destack/view",
                customConditions: conditions,
                paths,
            },
            files,
            include: [
                resolve(source.directory, "src/**/*.ts"),
                resolve(source.directory, "src/**/*.tsx"),
                resolve(source.directory, "test/**/*.test.ts"),
                resolve(source.directory, "tests/**/*.test.ts"),
            ],
        };
    }
}
