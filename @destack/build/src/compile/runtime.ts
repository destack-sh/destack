import type { BuildDescription } from "@destack/package/inspect";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Toolchain } from "@destack/check/toolchain";
import { API, type Program } from "typescript/unstable/async";
import type { GlobalReference, ModuleDescription } from "@destack/package/code";
import type { Runtime } from "@destack/package/runtime";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import { BuildError } from "../error/index.ts";
import { modulePath } from "./dependency.ts";
import { TypeScriptCompiler } from "../typescript/compiler.ts";

/** This package's directory, whose dependencies hold the runtime declarations when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

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

        // write one access per distinct global for the compiler to resolve
        const expressions = accessExpressions(references, runtime);
        const lines = [...expressions.keys()];
        const source = lines.join("\n") + "\nexport {};\n";
        const { directory, api, configuration } = await this.#open(runtime);
        const entry = join(directory, "runtime.ts");
        await writeFile(entry, source);
        api.clearSourceFileCache();

        // resolve each used global and member through the runtime's type environment
        const snapshot = await api.updateSnapshot({
            openProjects: [configuration],
            fileChanges: { invalidateAll: true },
        });
        try {
            const project = snapshot.getProject(configuration);
            if (!project) {
                throw new BuildError("BUILD_FAILED", `missing ${runtime} type environment`);
            }
            await checkEnvironment(project.program, runtime);

            // report rejected APIs at their original source locations
            const failures = await project.program.getSemanticDiagnostics();
            for (const failure of failures) {
                if (failure.fileName !== entry) {
                    throw new BuildError("BUILD_FAILED", `invalid ${runtime} type environment`, {
                        cause: failures,
                    });
                }

                // translate the generated location back to the authored access
                const line = source.slice(0, failure.pos).split("\n").length - 1;
                const generated = lines[line];
                const reference = generated === undefined ? undefined : expressions.get(generated);
                if (!reference) {
                    throw new BuildError("BUILD_FAILED", failure.text, { cause: failure });
                }
                throw unsupported(reference, runtime);
            }
        } finally {
            await snapshot.dispose();
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
            const entry = join(directory, "runtime.ts");
            await writeFile(entry, "export {};\n");
            const configuration = join(directory, "tsconfig.json");
            const libraries =
                runtime === "browser" ? ["ESNext", "DOM", "DOM.Iterable"] : ["ESNext"];
            await writeFile(
                configuration,
                JSON.stringify({
                    compilerOptions: { ...TYPESCRIPT_OPTIONS, types: [], lib: libraries },
                    files: [entry, ...declarationFiles(runtime)],
                }),
            );

            // retain the compiler with its temporary configuration
            const api = new API({ cwd: directory, tsserverPath: TypeScriptCompiler.executable() });
            const environment = { directory, configuration, api };
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
            .map((result): unknown => result.reason);
        if (failures.length) {
            throw new AggregateError(failures, "runtime compiler shutdown failed");
        }
    }
}

/** Write one access expression per distinct global, refusing dynamic access and code evaluation on workerd. */
function accessExpressions(
    references: readonly GlobalReference[],
    runtime: Runtime,
): Map<string, GlobalReference> {
    const expressions = new Map<string, GlobalReference>();
    for (const reference of references) {
        // refuse dynamic property access and code evaluation on workerd
        if (reference.dynamic || (runtime === "workerd" && isEvaluation(reference))) {
            throw unsupported(reference, runtime);
        }

        // access each member without narrowing on the way
        const members = reference.members.map((member) => `[${JSON.stringify(member)}]!`);
        expressions.set(`void (${reference.name})!${members.join("")};`, reference);
    }

    return expressions;
}

/** Report whether a global reaches `eval` or the `Function` constructor, which workerd refuses to run. */
function isEvaluation(reference: GlobalReference): boolean {
    // read the access path from the global object
    const path =
        reference.name === "globalThis"
            ? reference.members
            : [reference.name, ...reference.members];
    const [name, member, prototypeMember] = path;

    // accept the function prototype's members except its constructor
    const isPrototype = member === "prototype" && prototypeMember !== "constructor";

    return name === "eval" || (name === "Function" && !isPrototype);
}

/** Require the runtime configuration to parse and its declarations to check. */
async function checkEnvironment(program: Program, runtime: Runtime): Promise<void> {
    const diagnostics = [
        ...(await program.getConfigFileParsingDiagnostics()),
        ...(await program.getSyntacticDiagnostics()),
        ...(await program.getGlobalDiagnostics()),
    ];
    if (diagnostics.length) {
        const messages = diagnostics.map(
            (diagnostic) => `${diagnostic.fileName ?? runtime}: ${diagnostic.text}`,
        );
        throw new BuildError(
            "BUILD_FAILED",
            `invalid ${runtime} type environment:\n${messages.join("\n")}`,
            {
                cause: diagnostics,
            },
        );
    }
}

/** List the declaration files of a runtime's global APIs, none for the browser's built-in ones. */
function declarationFiles(runtime: Runtime): string[] {
    // load the Workers types
    if (runtime === "workerd") {
        return [join(Toolchain.locate(["@cloudflare/workers-types"], PACKAGE), "index.d.ts")];
    }
    // load the Bun types
    else if (runtime === "bun") {
        return [join(Toolchain.locate(["@types/bun"], PACKAGE), "index.d.ts")];
    }
    // load nothing beyond the libraries
    else {
        return [];
    }
}

/** Report an unavailable API at the authored expression. */
function unsupported(reference: GlobalReference, runtime: Runtime): BuildError {
    const path = [reference.name, ...reference.members].join(".");

    return new BuildError(
        "BUILD_FAILED",
        `unsupported ${runtime} API: ${path} at ${reference.source.file}:${reference.source.start}`,
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
            throw new BuildError("BUILD_FAILED", `unknown compiled module: ${id}`);
        }
        if (input.runtimes && !input.runtimes.includes(runtime)) {
            throw new BuildError(
                "BUILD_FAILED",
                `unsupported ${runtime} module: ${input.package ?? "source"}/${input.path}`,
            );
        }
        if (input.unresolvedImports !== undefined && input.unresolvedImports.length > 0) {
            throw new BuildError("BUILD_FAILED", `unresolved runtime imports: ${input.path}`);
        }

        // dependency source declarations remain in the compiler list under their package
        if (input.package === undefined) {
            globals.push(...(source.get(modulePath(input.path))?.globals ?? []));
        }
    }

    // check the collected globals against the runtime
    await compiler.check(globals, runtime);
}
