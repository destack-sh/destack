import * as test from "@destack/test/config";
import { loadTransforms } from "@destack/build/local";
import type { Environment } from "@destack/environment";
import { builtinModules } from "node:module";
import { fileURLToPath } from "node:url";

/** How a test configuration compiles components beside its DOM. */
export interface ViewTestOptions {
    /** Compile components to hydrate pages a server rendered. */
    readonly isHydrated?: boolean;
    /** Play scenarios in Chromium. */
    readonly isBrowser?: boolean;
}

/** Define a test configuration that compiles modules as builds do and renders them into a DOM. */
export function defineConfiguration(
    configuration: Extract<test.TestConfiguration, { readonly tier: test.TestTier }>,
    environment: Environment<"files" | "process">,
    options: ViewTestOptions = {},
): ReturnType<typeof test.defineConfiguration> {
    // transform modules as builds do, for the server outside the DOM and to hydrate when asked
    const root = configuration.root ?? environment.process.workingDirectory;
    const isBrowser = options.isBrowser === true;
    const isDom = configuration.test?.environment === undefined && !isBrowser;
    const drivers = fileURLToPath(new URL(isBrowser ? "page.ts" : "index.ts", import.meta.url));
    const setup = fileURLToPath(new URL("chromium.ts", import.meta.url));
    const transforms = loadTransforms({
        directory: root,
        runtime: isDom ? "browser" : "bun",
        server: !isDom || options.isHydrated === true,
        environment,
    });

    // render components in a DOM, loading the runtime's modules natively for test servers
    return test.defineConfiguration(
        {
            ...configuration,
            plugins: [
                {
                    name: "destack-runtime-modules",
                    enforce: "pre",
                    resolveId: (id) =>
                        /^(?:node|bun):/u.test(id) || builtinModules.includes(id)
                            ? { id, external: true }
                            : undefined,
                },
                transforms,
                ...(isDom || isBrowser ? [test.scenarioPlugin([drivers])] : []),
                ...(configuration.plugins ?? []),
            ],
            test: {
                environment: isBrowser
                    ? "node"
                    : fileURLToPath(new URL("environment.ts", import.meta.url)),
                ...configuration.test,
                ...(isBrowser
                    ? { globalSetup: [configuration.test?.globalSetup ?? [], setup].flat() }
                    : {}),
            },
        },
        environment,
    );
}
