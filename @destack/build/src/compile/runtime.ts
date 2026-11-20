import { LISTED_MODULE_RUNTIMES, type Runtime } from "@destack/package/runtime";
import type { BuildDescription } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";

/** Refuse compiled modules an output's runtime cannot load. */
export function checkInputs(build: BuildDescription, runtime: Runtime): void {
    for (const id of new Set(Object.values(build.outputs).flatMap((output) => output.inputs))) {
        // require each compiled module
        const input = build.inputs[id];
        if (!input) {
            throw BuildError.of("INVARIANT_BROKEN", `unknown compiled module: '${id}'`);
        }
        // refuse a module of other runtimes
        else if (input.runtimes && !input.runtimes.includes(runtime)) {
            throw BuildError.of(
                "RUNTIME_MODULE_INVALID",
                `module '${input.package ?? "source"}/${input.path}' is invalid: it is not supported on '${runtime}'`,
            );
        }
        // refuse computed imports on a runtime loading only the modules its deployment lists
        else if (
            input.unresolvedImports !== undefined &&
            input.unresolvedImports.length > 0 &&
            LISTED_MODULE_RUNTIMES.has(runtime)
        ) {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `runtime import set of '${input.path}' is invalid: it is unresolved`,
            );
        }
    }
}
