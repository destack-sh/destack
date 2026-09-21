import { type ModuleDescription } from "@destack/package/code";
import { type BuildDescription } from "@destack/package/inspect";
import { type Runtime } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";
import type { RuntimeCompiler } from "../inspect/runtime.ts";
import { isBuiltin } from "node:module";

/** Check emitted source against declared runtime support and the runtime's global APIs. */
export async function checkRuntime(
    build: BuildDescription,
    modules: readonly ModuleDescription[],
    runtime: Runtime,
    compiler: RuntimeCompiler,
): Promise<void> {
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
        case "deno":
            return ["server", "deno", "node", "module", mode];
        case "workerd":
            return ["server", "worker", "workerd", "module", mode];
    }
}

/** Identify built-in modules supplied by the selected Destack runtime. */
export function isRuntimeModule(specifier: string, runtime: Runtime): boolean {
    return (
        (runtime === "deno" && isBuiltin(specifier)) ||
        (runtime === "workerd" && specifier === "node:async_hooks")
    );
}

/** Preserve supported built-ins and reject unsupported host module imports. */
export function externalModule(specifier: string, runtime: Runtime): boolean {
    if (!isBuiltin(specifier)) {
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
