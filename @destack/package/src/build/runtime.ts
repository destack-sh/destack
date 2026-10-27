import { isBuiltin } from "node:module";
import { Features } from "lightningcss";
import type { CSSOptions } from "vite";
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

/** The browsers Destack builds for: Baseline widely available on 2026-10-27, engines of 2024-04. */
export const BROWSER_TARGETS: readonly string[] = [
    "chrome124",
    "edge124",
    "firefox125",
    "safari17.4",
];

/** Select the syntax a runtime compiles to: the baseline browsers', or current syntax elsewhere. */
export function runtimeTarget(runtime: Runtime): string[] {
    return runtime === "browser" ? [...BROWSER_TARGETS] : ["esnext"];
}

/** Select how a runtime's stylesheets compile: lowered for the baseline browsers, else as written. */
export function runtimeCss(runtime: Runtime): CSSOptions {
    // keep light-dark() native, as the theme switches appearance at runtime where lowering cannot follow
    return runtime === "browser"
        ? {
              transformer: "lightningcss",
              lightningcss: { targets: cssTargets(), exclude: Features.LightDark },
          }
        : {};
}

/** Encode the baseline browsers as Lightning CSS targets, each version as major and minor bytes. */
function cssTargets(): Record<string, number> {
    return Object.fromEntries(
        BROWSER_TARGETS.map((target) => {
            // split a target such as "safari17.4" into its browser and version
            const match = /^([a-z]+)(\d+)(?:\.(\d+))?$/u.exec(target);
            if (match === null) {
                throw new TypeError(`unknown browser target: ${target}`);
            }
            const [, browser = "", major = "0", minor = "0"] = match;

            return [browser, (Number(major) << 16) | (Number(minor) << 8)];
        }),
    );
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
