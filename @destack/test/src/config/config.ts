import { existsSync, globSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as vitest from "vitest/config";
import type { UserWorkspaceConfig, ViteUserConfig } from "vitest/config";
import type { Plugin } from "vite";
import { modulePlugin } from "@destack/package/vite";
import { schema } from "@destack/schema";

/** How often `expect.poll` rechecks, in milliseconds: a local sync round trip takes a few, so the default 50 only adds idle waiting. */
const POLL = { poll: { interval: 5 } };

/** The workspace patterns of a package.json. */
const Manifest = schema.looseObject({ workspaces: schema.array(schema.string()).exactOptional() });

/** Define a test configuration whose package sources receive Destack module metadata. */
export function defineConfiguration(configuration: ViteUserConfig): ViteUserConfig {
    return vitest.defineConfig({
        ...configuration,
        test: { fsModuleCache: true, expect: POLL, ...configuration.test },
        plugins: [modulePlugin(), cachePlugin(), ...(configuration.plugins ?? [])],
    });
}

/** Define a test project whose package sources receive Destack module metadata. */
export function defineProject(configuration: UserWorkspaceConfig): UserWorkspaceConfig {
    return vitest.defineProject({
        ...configuration,
        test: { fsModuleCache: true, expect: POLL, ...configuration.test },
        plugins: [modulePlugin(), cachePlugin(), ...(configuration.plugins ?? [])],
    });
}

/** The modules declaring a package's scenarios, which the runner plays as tests. */
export const SCENARIO_MODULES = "src/**/*.scenario.{ts,tsx}";

/** A driver type a package's scenarios play through, as its module and export. */
export interface DriverReference {
    /** The module exporting the driver type, such as `@destack/view/test`. */
    readonly module: string;
    /** The export, such as `DomDriver`. */
    readonly name: string;
}

/** Play each scenario a scenario module exports as a test of that module, through the driver of its interaction. */
export function scenarioPlugin(drivers: readonly DriverReference[]): Plugin {
    return {
        name: "destack-scenarios",
        transform: {
            filter: { id: /\.scenario\.tsx?$/u },
            handler(code, id) {
                // import the module itself, the runner and the driver types, then play what it exports
                const self = JSON.stringify(`./${basename(id)}`);
                const imported = drivers.map(
                    (driver, index) =>
                        `import { ${driver.name} as __DestackDriver${index} } from ${JSON.stringify(driver.module)};`,
                );
                const types = drivers.map((_, index) => `__DestackDriver${index}`).join(", ");
                const played = [
                    `import { Runner as __DestackRunner } from "@destack/test";`,
                    `import * as __destackScenarios from ${self};`,
                    ...imported,
                    `__DestackRunner.playModule(__destackScenarios, [${types}]);`,
                ];

                return { code: `${code}\n${played.join("\n")}\n`, map: null };
            },
        },
    };
}

/** Key cached module transforms by every package definition the module transform reads, and by its own sources. */
function cachePlugin(): Plugin {
    let fingerprint: string | undefined;

    return {
        name: "destack-module-cache",
        configureVitest({ defineCacheKeyGenerator }) {
            defineCacheKeyGenerator(() => (fingerprint ??= definitions()));
        },
    };
}

/** Digest the workspace's package definitions and the module transform's sources. */
function definitions(): string {
    // find the workspace root listing the packages
    let root = process.cwd();
    let workspaces = readWorkspaces(root);
    while (workspaces === undefined) {
        if (dirname(root) === root) {
            throw new TypeError(`no workspace above ${process.cwd()}`);
        }
        root = dirname(root);
        workspaces = readWorkspaces(root);
    }

    // read each package's destack.json and package.json, skipping dependencies and hidden directories
    const packages = globSync(
        workspaces.flatMap((pattern) => [
            `${pattern}/**/destack.json`,
            `${pattern}/**/package.json`,
        ]),
        {
            cwd: root,
            exclude: (path) => basename(path) === "node_modules" || basename(path).startsWith("."),
        },
    ).map((file) => join(root, file));

    // read the module transform's sources and its Vite plugin
    const transform = dirname(fileURLToPath(import.meta.resolve("@destack/package/transform")));
    const sources = [
        ...globSync("*.ts", { cwd: transform }).map((file) => join(transform, file)),
        fileURLToPath(import.meta.resolve("@destack/package/vite")),
    ];
    const files = [...packages, ...sources].toSorted();

    // hash each file's path and content
    const hash = createHash("sha256");
    for (const file of files) {
        hash.update(file).update("\0").update(readFileSync(file)).update("\0");
    }

    return hash.digest("hex");
}

/** Read the workspace patterns a directory's package.json lists, if it lists any. */
function readWorkspaces(directory: string): string[] | undefined {
    const file = join(directory, "package.json");
    if (!existsSync(file)) {
        return undefined;
    }

    return Manifest.parse(JSON.parse(readFileSync(file, "utf8"))).workspaces;
}
