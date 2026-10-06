import * as test from "@destack/test/config";
import { loadTransforms } from "@destack/build/local";
import type { PluginOption } from "@destack/package/build";
import { builtinModules } from "node:module";
import { fileURLToPath } from "node:url";

/** Define a test configuration that compiles modules as builds do and renders them into a DOM. */
export function defineConfiguration(
    configuration: Parameters<typeof test.defineConfiguration>[0],
): ReturnType<typeof test.defineConfiguration> {
    // transform modules as builds do, leaving out the development server's hooks
    const root = configuration.root ?? process.cwd();
    const transforms = loadTransforms({
        directory: root,
        runtime: "browser",
        server: false,
        options: {},
    }).then(async (plugins) => await Promise.all(plugins.map(withoutServer)));

    // play the package's scenarios through the DOM driver when the tests render into the DOM
    const isDom = configuration.test?.environment === undefined;
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

/** Drop the development server hook of each plugin, whose timers would outlive the tests. */
async function withoutServer(option: PluginOption): Promise<PluginOption> {
    // descend into nested and pending options
    const settled = await option;
    if (Array.isArray(settled)) {
        return await Promise.all(settled.map(withoutServer));
    }
    // keep disabled options as they are
    else if (settled === false || settled === null || settled === undefined) {
        return settled;
    }

    return { ...settled, configureServer: undefined };
}
