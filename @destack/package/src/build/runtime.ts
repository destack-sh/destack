import { isBuiltin } from "node:module";
import type { Runtime } from "../runtime/index.ts";

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

/** Identify modules a host supplies rather than a package: built-ins and Bun's own. */
export function isHostModule(specifier: string): boolean {
    return isBuiltin(specifier) || specifier === "bun" || specifier.startsWith("bun:");
}

/** Identify host modules the selected Destack runtime supplies. */
export function isRuntimeModule(specifier: string, runtime: Runtime): boolean {
    return (
        (runtime === "bun" && isHostModule(specifier)) ||
        (runtime === "workerd" && specifier === "node:async_hooks")
    );
}
