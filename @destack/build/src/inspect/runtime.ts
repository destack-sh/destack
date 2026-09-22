import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { API } from "typescript/unstable/async";
import type { GlobalReference } from "@destack/package/code";
import type { Runtime } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";

/** Resolve installed runtime declarations from the compiler package. */
const REQUIRE = createRequire(import.meta.url);

/** Runtime type environments retained across builds of one package. */
export class RuntimeCompiler implements AsyncDisposable {
    /** Initialized runtime configurations and their native compilers. */
    readonly #environments = new Map<
        Runtime,
        { directory: string; configuration: string; api: API }
    >();

    /** Check globals against the selected runtime's declarations. */
    async check(references: readonly GlobalReference[], runtime: Runtime): Promise<void> {
        if (!references.length) {
            return;
        }

        // deduplicate accessed globals and reject dynamic property access
        const expressions = new Map<string, GlobalReference>();
        for (const reference of references) {
            const name = reference.name === "globalThis" ? reference.members[0] : reference.name;
            if (
                reference.dynamic ||
                (runtime === "workerd" && (name === "eval" || name === "Function"))
            ) {
                throw unsupported(reference, runtime);
            }

            const expression = `void (${reference.name})!${reference.members
                .map((member) => `[${JSON.stringify(member)}]!`)
                .join("")};`;
            expressions.set(expression, reference);
        }

        // let the compiler resolve named members, index signatures, and inherited properties
        const lines = [...expressions.keys()];
        const source = lines.join("\n") + "\nexport {};\n";
        const environment = await this.#open(runtime);
        const { directory, api, configuration } = environment;
        {
            // create an empty module for subsequent global checks
            const entry = join(directory, "runtime.ts");
            await writeFile(entry, source);
            api.clearSourceFileCache();

            // resolve each used global and member through the runtime's own type environment
            const snapshot = await api.updateSnapshot({
                openProjects: [configuration],
                fileChanges: { invalidateAll: true },
            });
            try {
                const project = snapshot.getProject(configuration);
                if (!project) {
                    throw new BuildError("BUILD_FAILED", `Missing ${runtime} type environment.`);
                }

                // check the isolated runtime configuration
                const diagnostics = [
                    ...(await project.program.getConfigFileParsingDiagnostics()),
                    ...(await project.program.getSyntacticDiagnostics()),
                    ...(await project.program.getGlobalDiagnostics()),
                ];
                if (diagnostics.length) {
                    throw new BuildError("BUILD_FAILED", `Invalid ${runtime} type environment.`, {
                        cause: diagnostics,
                    });
                }

                // report rejected APIs at their original source locations
                const failures = await project.program.getSemanticDiagnostics();
                for (const failure of failures) {
                    if (failure.fileName !== entry) {
                        throw new BuildError(
                            "BUILD_FAILED",
                            `Invalid ${runtime} type environment.`,
                            {
                                cause: failures,
                            },
                        );
                    }

                    // translate the generated location back to the authored access
                    const line = source.slice(0, failure.pos).split("\n").length - 1;
                    const reference = expressions.get(lines[line]);
                    if (!reference) {
                        throw new BuildError("BUILD_FAILED", failure.text, { cause: failure });
                    }
                    throw unsupported(reference, runtime);
                }
            } finally {
                await snapshot.dispose();
            }
        }
    }

    /** Initialize each runtime once without importing application ambient declarations. */
    async #open(runtime: Runtime) {
        const existing = this.#environments.get(runtime);
        if (existing) {
            return existing;
        }

        // create the runtime configuration outside the source package
        const directory = await mkdtemp(join(tmpdir(), "destack-runtime-"));
        try {
            // load runtime APIs independently of application declarations and dependency typings
            const files: string[] = [];
            if (runtime === "workerd") {
                files.push(
                    join(
                        dirname(REQUIRE.resolve("@cloudflare/workers-types/package.json")),
                        "index.d.ts",
                    ),
                );
            } else if (runtime === "bun") {
                files.push(REQUIRE.resolve("@types/bun/index.d.ts"));
            }
            const entry = join(directory, "runtime.ts");
            await writeFile(entry, "export {};\n");
            const configuration = join(directory, "tsconfig.json");
            await writeFile(
                configuration,
                JSON.stringify({
                    compilerOptions: {
                        target: "ESNext",
                        module: "ESNext",
                        moduleResolution: "Bundler",
                        types: [],
                        lib: runtime === "browser" ? ["ESNext", "DOM", "DOM.Iterable"] : ["ESNext"],
                        skipLibCheck: true,
                        strict: true,
                        noEmit: true,
                    },
                    files: [entry, ...files],
                }),
            );

            // retain the compiler with its temporary configuration
            const environment = { directory, configuration, api: new API({ cwd: directory }) };
            this.#environments.set(runtime, environment);

            return environment;
        } catch (error) {
            await rm(directory, { recursive: true });
            throw error;
        }
    }

    /** Close native compilers and remove their temporary configurations. */
    async [Symbol.asyncDispose](): Promise<void> {
        // close each compiler before deleting its files
        const results = await Promise.allSettled(
            [...this.#environments.values()].map(async ({ api, directory }) => {
                try {
                    await api.close();
                } finally {
                    await rm(directory, { recursive: true });
                }
            }),
        );

        // report every failed shutdown
        this.#environments.clear();
        const failures = results
            .filter((result) => result.status === "rejected")
            .map((result) => result.reason);
        if (failures.length) {
            throw new AggregateError(failures, "runtime compiler shutdown failed");
        }
    }
}

/** Report an unavailable API at the authored expression. */
function unsupported(reference: GlobalReference, runtime: Runtime): BuildError {
    const path = [reference.name, ...reference.members].join(".");

    return new BuildError(
        "BUILD_FAILED",
        `Unsupported ${runtime} API: ${path} at ${reference.source.file}:${reference.source.start}`,
    );
}
