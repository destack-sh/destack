import { isBuiltin } from "node:module";
import { nth } from "@destack/schema";
import type { Runtime } from "../runtime/index.ts";

/** The workerd compatibility date Destack's Workers and workloads run at. */
export const WORKERD_COMPATIBILITY_DATE = "2026-07-30";

/** Workerd's Node modules then, per developers.cloudflare.com/workers/runtime-apis/nodejs. */
const WORKERD_NODE_MODULES: ReadonlySet<string> = new Set([
    "_stream_wrap",
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "sqlite",
    "stream",
    "string_decoder",
    "test",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
]);

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
        // resolve as Wrangler does, falling back to a package's browser entry over its Node one
        case "workerd":
            return ["server", "worker", "workerd", "browser", "module", mode];
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
        (runtime === "workerd" &&
            isBuiltin(specifier) &&
            WORKERD_NODE_MODULES.has(nth(specifier.replace(/^node:/u, "").split("/"), 0)))
    );
}

/** The type checks every package compiles under, the same as Destack's sources and templates. */
export const TYPE_CHECKS = {
    strict: true,
    noUncheckedIndexedAccess: true,
    exactOptionalPropertyTypes: true,
    noImplicitOverride: true,
    noPropertyAccessFromIndexSignature: true,
    noImplicitReturns: true,
    noFallthroughCasesInSwitch: true,
    allowUnreachableCode: false,
    allowUnusedLabels: false,
    noUncheckedSideEffectImports: true,
    erasableSyntaxOnly: true,
    verbatimModuleSyntax: true,
} as const;

/** The compiler options every package compiles under, its type checks included. */
export const TYPESCRIPT_OPTIONS = {
    target: "ESNext",
    module: "Preserve",
    moduleResolution: "Bundler",
    allowImportingTsExtensions: true,
    noEmit: true,
    skipLibCheck: true,
    ...TYPE_CHECKS,
} as const;
