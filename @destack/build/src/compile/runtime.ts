import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Toolchain } from "@destack/check/toolchain";
import { API, type Program } from "typescript/unstable/async";
import type { GlobalReference, ModuleDescription } from "@destack/package/code";
import { FILE_RESOLVING_RUNTIMES, type Runtime } from "@destack/package/runtime";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import type { BuildDescription } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";
import { modulePath } from "./dependency.ts";
import { TypeScriptCompiler } from "../typescript/compiler.ts";

/** This package's directory, whose dependencies have runtime declarations in a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** Runtime type environments retained across builds of one package. */
export class RuntimeCompiler implements AsyncDisposable {
    /** Initialized runtime configurations and their native compilers. */
    readonly #environments = new Map<
        Runtime,
        { directory: string; configuration: string; api: API }
    >();

    /** Refuse each global that every runtime a module ships to lacks. */
    async check(
        references: readonly GlobalReference[],
        runtimes: readonly Runtime[],
    ): Promise<void> {
        // refuse dynamic property access, which no runtime's declarations check
        const dynamic = references.find((reference) => reference.dynamic);
        if (dynamic !== undefined) {
            const { path, location } = describeReference(dynamic);
            throw new BuildError("BUILD_FAILED", `dynamic global access: ${path} at ${location}`);
        }

        // collect the globals each runtime rejects
        const rejected = await Promise.all(
            runtimes.map(async (runtime) => new Set(await this.#rejected(references, runtime))),
        );

        // refuse the first global none of them has
        const refused = references.find((reference) =>
            rejected.every((each) => each.has(reference)),
        );
        if (refused !== undefined) {
            throw unsupported(refused, runtimes);
        }
    }

    /** List the globals a runtime's declarations lack. */
    async #rejected(
        references: readonly GlobalReference[],
        runtime: Runtime,
    ): Promise<GlobalReference[]> {
        if (!references.length) {
            return [];
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

            // list rejected APIs at their original source locations
            const failures = await project.program.getSemanticDiagnostics();
            const rejected: GlobalReference[] = [];
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
                rejected.push(reference);
            }

            return rejected;
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

/** Write one access expression per global, refusing eval on workerd. */
function accessExpressions(
    references: readonly GlobalReference[],
    runtime: Runtime,
): Map<string, GlobalReference> {
    const expressions = new Map<string, GlobalReference>();
    for (const reference of references) {
        // refuse code evaluation on workerd
        if (runtime === "workerd" && isEvaluation(reference)) {
            throw unsupported(reference, [runtime]);
        }

        // access each member without narrowing on the way
        const members = reference.members.map((member) => `[${JSON.stringify(member)}]!`);
        expressions.set(`void (${reference.name})!${members.join("")};`, reference);
    }

    return expressions;
}

/** Report whether a global is `eval` or the `Function` constructor, which workerd refuses. */
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

/** Report an API none of a module's runtimes has, at the authored expression. */
function unsupported(reference: GlobalReference, runtimes: readonly Runtime[]): BuildError {
    const { path, location } = describeReference(reference);

    return new BuildError(
        "BUILD_FAILED",
        `unsupported ${runtimes.join(", ")} API: ${path} at ${location}`,
    );
}

/** Describe a global reference by its member path and its source location. */
function describeReference(reference: GlobalReference): { path: string; location: string } {
    const path = [reference.name, ...reference.members].join(".");
    const location = `${reference.source.file}:${reference.source.start}`;

    return { path, location };
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
    const evaluated: GlobalReference[] = [];
    const deferred = new Map<
        string,
        { runtimes: readonly Runtime[]; globals: GlobalReference[] }
    >();

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
        // refuse imports a deployed module graph lacks, unless the runtime resolves them from files
        const isUnresolved =
            input.unresolvedImports !== undefined && input.unresolvedImports.length > 0;
        if (isUnresolved && !FILE_RESOLVING_RUNTIMES.has(runtime)) {
            throw new BuildError("BUILD_FAILED", `unresolved runtime imports: ${input.path}`);
        }

        // collect the package's globals, those in functions by the runtimes their module ships to
        if (input.package === undefined) {
            const runtimes = input.runtimes ?? [runtime];
            const key = runtimes.toSorted().join(" ");
            const group = deferred.get(key) ?? { runtimes, globals: [] };
            for (const global of source.get(modulePath(input.path))?.globals ?? []) {
                (global.evaluation === "module" ? evaluated : group.globals).push(global);
            }
            deferred.set(key, group);
        }
    }

    // check the globals module evaluation runs against the output's runtime
    await compiler.check(evaluated, [runtime]);

    // check the globals in functions against any runtime their module ships to
    for (const group of deferred.values()) {
        await compiler.check(group.globals, group.runtimes);
    }
}
