import * as test from "@destack/test/config";
import { loadTransforms } from "@destack/build/local";
import { builtinModules } from "node:module";
import { fileURLToPath } from "node:url";

/** How a test configuration compiles components beside its DOM. */
export interface ViewTestOptions {
    /** Compile components to hydrate pages a server rendered, as builds of server-rendered applications do. */
    readonly isHydrated?: boolean;
}

/** Define a test configuration that compiles modules as builds do and renders them into a DOM. */
export function defineConfiguration(
    configuration: Parameters<typeof test.defineConfiguration>[0],
    options: ViewTestOptions = {},
): ReturnType<typeof test.defineConfiguration> {
    // transform modules as builds do, for the server outside the DOM and to hydrate when asked
    const root = configuration.root ?? process.cwd();
    const isDom = configuration.test?.environment === undefined;
    const transforms = loadTransforms({
        directory: root,
        runtime: isDom ? "browser" : "bun",
        server: !isDom || options.isHydrated === true,
        options: {},
    });

    // play the package's scenarios through the DOM driver when the tests render into the DOM
    const include = configuration.test?.include ?? test.configDefaults.include;

    // render components in a DOM, loading the runtime's modules natively for test servers
    return test.defineConfiguration({
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
            ...(isDom
                ? [test.scenarioPlugin([{ module: "@destack/view/test", name: "ViewDriver" }])]
                : []),
            ...(configuration.plugins ?? []),
        ],
        test: {
            environment: fileURLToPath(new URL("environment.ts", import.meta.url)),
            ...configuration.test,
            include: isDom ? [...include, test.SCENARIO_MODULES] : include,
        },
    });
}
