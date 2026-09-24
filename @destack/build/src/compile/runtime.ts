import type { BuildDescription } from "@destack/package/inspect";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createRequire, isBuiltin } from "node:module";
import { API } from "typescript/unstable/async";
import type { GlobalReference, ModuleDescription } from "@destack/package/code";
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
        // reuse an opened runtime environment
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

/** Check emitted source against the declared runtime APIs and the runtime's global APIs. */
export async function checkRuntime(
    build: BuildDescription,
    modules: readonly ModuleDescription[],
    runtime: Runtime,
    compiler: RuntimeCompiler,
): Promise<void> {
    // index the compiled inputs and source modules
    const inputs = new Set(Object.values(build.outputs).flatMap((output) => output.inputs));
    const source = new Map(modules.map((module) => [module.path, module]));
    const globals = [];

    // check retained modules once, including library outputs without workload declarations
    for (const id of inputs) {
        const input = build.inputs[id];
        if (!input) {
            throw new BuildError("BUILD_FAILED", `Unknown compiled module: ${id}`);
        }
        if (input.runtimes && !input.runtimes.includes(runtime)) {
            throw new BuildError(
                "BUILD_FAILED",
                `Unsupported ${runtime} module: ${input.package ?? "source"}/${input.path}`,
            );
        }
        if (input.unresolvedImports?.length) {
            throw new BuildError("BUILD_FAILED", `Unresolved runtime imports: ${input.path}`);
        }

        // dependency source declarations remain in the compiler inventory under their package
        if (!input.package) {
            globals.push(...(source.get(input.path.split("?")[0])?.globals ?? []));
        }
    }

    // check the collected globals against the runtime
    await compiler.check(globals, runtime);
}

/** Select standard package conditions for a runtime and build mode. */
export function runtimeConditions(
    runtime: Runtime,
    mode: "development" | "production" = "production",
): string[] {
    switch (runtime) {
        case "browser":
            return ["browser", "module", mode];
        case "bun":
            return ["server", "bun", "node", "module", mode];
        case "workerd":
            return ["server", "worker", "workerd", "module", mode];
    }
}

/** Identify built-in modules supplied by the selected Destack runtime. */
export function isRuntimeModule(specifier: string, runtime: Runtime): boolean {
    return (
        (runtime === "bun" &&
            (isBuiltin(specifier) || specifier === "bun" || specifier.startsWith("bun:"))) ||
        (runtime === "workerd" && specifier === "node:async_hooks")
    );
}

/** Preserve supported built-ins and reject unsupported host module imports. */
export function externalModule(specifier: string, runtime: Runtime): boolean {
    if (!isBuiltin(specifier) && specifier !== "bun" && !specifier.startsWith("bun:")) {
        return false;
    }
    if (!isRuntimeModule(specifier, runtime)) {
        throw new BuildError(
            "BUILD_FAILED",
            `Host module is unavailable on ${runtime}: ${specifier}`,
        );
    }

    return true;
}
