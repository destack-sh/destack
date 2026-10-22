import { existsSync, globSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as vitest from "vitest/config";
import type {
    TestProjectInlineConfiguration,
    UserWorkspaceConfig,
    ViteUserConfig,
} from "vitest/config";
import type { Plugin, PluginOption } from "vite";
import { modulePlugin } from "@destack/package/vite";
import { schema } from "@destack/schema";
import type { TestMatrix } from "../project/index.ts";

/** The shared test options: one module graph per worker, cached modules, and quick polls. */
const TEST = { isolate: false, fsModuleCache: true } as const;

/** How often `expect.poll` rechecks in milliseconds, a local sync round trip taking a few. */
const POLL = { poll: { interval: 5 } };

/** The workspace patterns and exports of a package.json. */
const Manifest = schema.looseObject({
    workspaces: schema.array(schema.string()).exactOptional(),
    exports: schema.record(schema.string(), schema.unknown()).exactOptional(),
});

/** The build extension a destack.json declares, such as `./build#viewExtension`. */
const Definition = schema.looseObject({ build: schema.string().exactOptional() });

/** A test project, and the targets it runs against. */
export type ProjectConfiguration = TestProjectInlineConfiguration & {
    /** The values it runs against by kind, one project per combination. */
    readonly targets?: TestMatrix;
};

/** A test configuration, its projects and the targets they run against. */
export type TestConfiguration = Omit<ViteUserConfig, "test"> & {
    /** The values its tests run against by kind, where its projects declare none. */
    readonly targets?: TestMatrix;
    /** The test options, whose projects may declare separate targets. */
    readonly test?: Omit<NonNullable<ViteUserConfig["test"]>, "projects"> & {
        /** The projects, each run once per combination of its targets. */
        readonly projects?: readonly ProjectConfiguration[];
    };
};

/** Define a test configuration whose package sources receive Destack module metadata. */
export function defineConfiguration(configuration: TestConfiguration): ViteUserConfig {
    // split off the targets, the projects and the global setup the projects run in the root's place
    const { targets, test, ...rest } = configuration;
    const { projects: declared, globalSetup, ...options } = test ?? {};
    const setup = globalSetup === undefined ? {} : { globalSetup };
    const projects = configuredProjects(declared, targets, setup);

    return vitest.defineConfig({
        ...rest,
        test: {
            ...TEST,
            expect: POLL,
            ...(projects === undefined ? { ...options, ...setup } : { ...options, projects }),
        },
        plugins: [modulePlugin(), cachePlugin(), ...(rest.plugins ?? []).map(withoutServer)],
    });
}

/** Define a test project whose package sources receive Destack module metadata. */
export function defineProject(configuration: UserWorkspaceConfig): UserWorkspaceConfig {
    return vitest.defineProject({
        ...configuration,
        test: { ...TEST, expect: POLL, ...configuration.test },
        plugins: [
            modulePlugin(),
            cachePlugin(),
            ...(configuration.plugins ?? []).map(withoutServer),
        ],
    });
}

/** The modules declaring a package's scenarios, which the runner plays as tests. */
export const SCENARIO_MODULES = "src/**/*.scenario.{ts,tsx}";

/** A driver type a package's scenarios play through, as its module and export. */
export interface DriverReference {
    /** The module exporting the driver type, such as `@destack/view/test`. */
    readonly module: string;
    /** The export, such as `ViewDriver`. */
    readonly name: string;
}

/** Play each exported scenario of a scenario module as a test of that module. */
export function scenarioPlugin(drivers: readonly DriverReference[]): Plugin {
    return {
        name: "destack-scenarios",
        transform: {
            filter: { id: /\.scenario\.tsx?$/u },
            handler(code, id) {
                // import the module itself, the runner and the driver types, then play its exports
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

/** Drop a plugin's development server hook. */
async function withoutServer(
    option: PluginOption,
): Promise<Exclude<PluginOption, Promise<unknown>>> {
    // descend into nested and pending options
    const settled = await option;
    if (Array.isArray(settled)) {
        return await Promise.all(settled.map(withoutServer));
    }
    // return disabled options as they are
    else if (settled === false || settled === null || settled === undefined) {
        return settled;
    }

    return { ...settled, configureServer: undefined };
}

/** List a configuration's projects, one per combination of targets, absent for the root alone. */
function configuredProjects(
    declared: readonly ProjectConfiguration[] | undefined,
    targets: TestMatrix | undefined,
    setup: Pick<NonNullable<TestProjectInlineConfiguration["test"]>, "globalSetup">,
): TestProjectInlineConfiguration[] | undefined {
    // run each declared project per combination, one extending the root with the root's setup
    if (declared !== undefined) {
        return declared.flatMap((project) =>
            targetProjects(
                project.extends === true
                    ? { ...project, test: { ...setup, ...project.test } }
                    : project,
                targets,
            ),
        );
    }
    // run the whole configuration per combination of its targets
    else if (targets !== undefined) {
        return targetProjects({ extends: true, test: setup }, targets);
    }
    // run the root alone
    else {
        return undefined;
    }
}

/** List a project once per combination of its targets, or the configuration's, named after them. */
function targetProjects(
    project: ProjectConfiguration,
    declared: TestMatrix | undefined,
): TestProjectInlineConfiguration[] {
    // return a project without targets as it is
    const { targets: local, ...config } = project;
    const targets = local ?? declared;
    if (targets === undefined) {
        return [config];
    }

    // list every combination of the targets' values
    const combinations = Object.entries(targets).reduce<Record<string, string>[]>(
        (listed, [kind, values]) =>
            listed.flatMap((combination) =>
                values.map((value) => ({ ...combination, [kind]: value })),
            ),
        [{}],
    );

    // run each combination as a project, named after the project and its values
    return combinations.map((combination) => {
        // title it by the project's label and the values, with its color
        const values = Object.values(combination).join(",");
        const named = config.test?.name;
        const name =
            named === undefined
                ? values
                : typeof named === "object"
                  ? { ...named, label: `${named.label}:${values}` }
                  : `${named}:${values}`;

        // provide the values to its tests
        return {
            ...config,
            test: {
                ...config.test,
                name,
                provide: { ...config.test?.provide, targets: combination },
            },
        };
    });
}

/** Key cached module transforms by every package definition the module transform reads. */
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

    // read each package's destack.json and package.json, skipping dependencies and hidden folders
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
    const extensions = packages
        .filter((file) => basename(file) === "destack.json")
        .flatMap((file) => extensionSources(dirname(file)));
    const files = [...packages, ...sources, ...extensions].toSorted();

    // hash each file's path and content
    const hash = createHash("sha256");
    for (const file of files) {
        hash.update(file).update("\0").update(readFileSync(file)).update("\0");
    }

    return hash.digest("hex");
}

/** List the sources of a package's build extension: the modules beside its build export. */
function extensionSources(directory: string): string[] {
    // read the build export the package's definition names, if it names one
    const definition = Definition.parse(
        JSON.parse(readFileSync(join(directory, "destack.json"), "utf8")),
    );
    if (definition.build === undefined) {
        return [];
    }
    const [subpath = ""] = definition.build.split("#", 1);
    const manifest = Manifest.parse(
        JSON.parse(readFileSync(join(directory, "package.json"), "utf8")),
    );
    const file = manifest.exports?.[subpath];
    if (typeof file !== "string") {
        throw new TypeError(`${directory} exports no build module at ${subpath}`);
    }

    // list the modules of the export's directory, its tests aside
    const root = dirname(join(directory, file));

    return globSync("**/*.{ts,tsx}", {
        cwd: root,
        exclude: (path) => path.endsWith(".test.ts"),
    }).map((path) => join(root, path));
}

/** Read the workspace patterns a directory's package.json lists, if it lists any. */
function readWorkspaces(directory: string): string[] | undefined {
    const file = join(directory, "package.json");
    if (!existsSync(file)) {
        return undefined;
    }

    return Manifest.parse(JSON.parse(readFileSync(file, "utf8"))).workspaces;
}
