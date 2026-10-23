import { realpathSync } from "node:fs";
import { realpath } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { API, type Diagnostic, type Project } from "typescript/unstable/async";
import { Toolchain } from "@destack/check/toolchain";
import { nth } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import { type InspectionDepth, TypeScriptInspection } from "./module.ts";
import { isAuthored } from "../source/dependency.ts";
import type { PackageSource } from "../source/index.ts";

/** This package's directory, whose dependencies include the tools when running from a workspace. */
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

    /** Inspect programs to a depth: check them at once, then read the first and the rest. */
    async inspect(
        programs: readonly Pick<
            PackageSource,
            "configuration" | "entries" | "directory" | "versions"
        >[],
        depth: InspectionDepth,
    ): Promise<TypeScriptInspection[]> {
        return await this.#read(
            programs.map((program) => program.configuration),
            async (projects) => {
                // check every program's configuration and authored modules at once
                const diagnostics = (
                    await Promise.all(projects.map((project) => this.#check(project)))
                ).flat();
                if (diagnostics.length) {
                    throw BuildError.of(
                        "INSPECTION_FAILED",
                        `the TypeScript inspection failed: ${JSON.stringify(diagnostics)}`,
                    );
                }

                // read the first program, then the others beside it
                const [first, ...rest] = programs.map((program, index) => ({
                    program,
                    project: nth(projects, index),
                }));
                if (first === undefined) {
                    return [];
                }
                const describing = TypeScriptInspection.read(first.project, first.program, depth);
                const [described, ...others] = await Promise.all([
                    describing,
                    ...rest.map(({ program, project }) =>
                        TypeScriptInspection.read(project, program, depth, describing),
                    ),
                ]);

                return [described, ...others];
            },
        );
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

    /** Parse a configuration into options and root files, without loading its program. */
    async configure(configuration: string): Promise<ParsedConfiguration> {
        const path = await realpath(resolve(this.directory, configuration));
        const parsed = await this.#api.parseConfigFile(path);

        return { options: parsed.options, files: parsed.fileNames };
    }

    /** Read configurations' projects from one fresh snapshot, then release the snapshot. */
    async #read<Result>(
        configurations: readonly string[],
        read: (projects: Project[]) => Promise<Result>,
    ): Promise<Result> {
        // refresh the selected configurations
        const paths = await Promise.all(
            configurations.map((configuration) => realpath(resolve(this.directory, configuration))),
        );

        // refresh client ASTs while the native compiler retains incremental project state
        this.#api.clearSourceFileCache();
        const invalidated = await this.#api.updateSnapshot({
            fileChanges: { invalidateAll: true },
        });
        await invalidated.dispose();

        // reread configured include patterns so added and removed modules change the lists
        const snapshot = await this.#api.updateSnapshot({
            openProjects: paths,
            fileChanges: { changed: paths },
        });

        // read the requested compiler projects and release the snapshot
        try {
            const projects = paths.map((path) => {
                const project = snapshot.getProject(path);
                if (!project) {
                    throw BuildError.of(
                        "INSPECTION_FAILED",
                        `the TypeScript compiler did not load ${path}`,
                    );
                }

                return project;
            });

            return await read(projects);
        } finally {
            await snapshot.dispose();
        }
    }

    /** Close the compiler and release retained projects. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#api.close();
    }
}
