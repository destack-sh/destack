import { realpathSync } from "node:fs";
import { realpath } from "node:fs/promises";
import { resolve } from "node:path";
import { API } from "typescript/unstable/async";
import { BuildError } from "../error/index.ts";
import { describeProject, type TypeScriptInspection } from "./module.ts";

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
        });
    }

    /** Inspect a configuration after notifying the compiler of changed files. */
    async inspect(configuration: string): Promise<TypeScriptInspection> {
        // refresh the selected configuration
        configuration = await realpath(resolve(this.directory, configuration));

        // refresh client ASTs while the native compiler retains incremental project state
        this.#api.clearSourceFileCache();
        const invalidated = await this.#api.updateSnapshot({
            fileChanges: { invalidateAll: true },
        });
        await invalidated.dispose();

        // reread configured include patterns so added and removed modules change the inventory
        const snapshot = await this.#api.updateSnapshot({
            openProjects: [configuration],
            fileChanges: { changed: [configuration] },
        });

        // inspect the project and release the snapshot
        try {
            // require the requested compiler project
            const project = snapshot.getProject(configuration);
            if (!project) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `TypeScript did not load ${configuration}`,
                );
            }

            // collect configuration, source, and type diagnostics together
            const diagnostics = (
                await Promise.all([
                    project.program.getConfigFileParsingDiagnostics(),
                    project.program.getSyntacticDiagnostics(),
                    project.program.getBindDiagnostics(),
                    project.program.getProgramDiagnostics(),
                    project.program.getGlobalDiagnostics(),
                    project.program.getSemanticDiagnostics(),
                ])
            ).flat();
            if (diagnostics.length) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `TypeScript inspection failed: ${JSON.stringify(diagnostics)}`,
                );
            }

            return await describeProject(project, this.directory);
        } finally {
            await snapshot.dispose();
        }
    }

    /** Close the compiler and release retained projects. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#api.close();
    }
}
