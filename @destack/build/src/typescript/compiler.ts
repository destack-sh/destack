import { realpathSync } from "node:fs";
import { realpath } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { API, type Project } from "typescript/unstable/async";
import { Toolchain } from "@destack/check/toolchain";
import { BuildError } from "../error/index.ts";
import { describeProject, type TypeScriptInspection } from "./module.ts";
import { isAuthored } from "../source/dependency.ts";
import type { ModulePackage } from "@destack/package/transform";

/** This package's directory, whose dependencies hold the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** A compiler configuration as the compiler parses it. */
export interface ParsedConfiguration {
    /** The effective compiler options. */
    readonly options: Readonly<Record<string, unknown>>;
    /** The root files of the program, by absolute path. */
    readonly files: readonly string[];
}

/** Retain TypeScript compiler state across inspections of one package. */
export class TypeScriptCompiler implements AsyncDisposable {
    /** The source package directory. */
    readonly directory: string;
    /** The compiler process and retained project state. */
    readonly #api: API;

    /** Start the native compiler for a source package. */
    constructor(directory: string) {
        this.directory = realpathSync(directory);
        this.#api = new API({
            cwd: this.directory,
            tsserverPath: TypeScriptCompiler.executable(),
        });
    }

    /** Find this platform's native TypeScript compiler among the tools. */
    static executable(): string {
        const platform = `@typescript/typescript-${process.platform}-${process.arch}`;

        return join(Toolchain.locate(["typescript", platform], PACKAGE), "lib", "tsc");
    }

    /** Inspect a configuration, collecting the declarations of the modules its entries import, the package as its build releases it. */
    inspect(
        configuration: string,
        entries: readonly string[],
        compiled: ModulePackage,
    ): Promise<TypeScriptInspection> {
        return this.#read(configuration, async (project) => {
            // check the configuration and the package's modules
            const authored = (await project.program.getSourceFileNames()).filter((file) =>
                isAuthored(this.directory, file),
            );
            const diagnostics = (
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
            if (diagnostics.length) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `the TypeScript inspection failed: ${JSON.stringify(diagnostics)}`,
                );
            }

            return await describeProject(project, compiled, entries);
        });
    }

    /** Parse a configuration into its effective options and root files, without loading its program. */
    async configure(configuration: string): Promise<ParsedConfiguration> {
        const path = await realpath(resolve(this.directory, configuration));
        const parsed = await this.#api.parseConfigFile(path);

        return { options: parsed.options, files: parsed.fileNames };
    }

    /** Read a configuration's project from a fresh snapshot, then release the snapshot. */
    async #read<Result>(
        configuration: string,
        read: (project: Project) => Promise<Result>,
    ): Promise<Result> {
        // refresh the selected configuration
        configuration = await realpath(resolve(this.directory, configuration));

        // refresh client ASTs while the native compiler retains incremental project state
        this.#api.clearSourceFileCache();
        const invalidated = await this.#api.updateSnapshot({
            fileChanges: { invalidateAll: true },
        });
        await invalidated.dispose();

        // reread configured include patterns so added and removed modules change the list
        const snapshot = await this.#api.updateSnapshot({
            openProjects: [configuration],
            fileChanges: { changed: [configuration] },
        });

        // read the requested compiler project and release the snapshot
        try {
            const project = snapshot.getProject(configuration);
            if (!project) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `the TypeScript compiler did not load ${configuration}`,
                );
            }

            return await read(project);
        } finally {
            await snapshot.dispose();
        }
    }

    /** Close the compiler and release retained projects. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#api.close();
    }
}
