import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as vitest from "vitest/config";
import type { TestProjectInlineConfiguration, ViteUserConfig } from "vitest/config";
import type { Plugin, PluginOption } from "vite";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import {
    type DirectoryEntry,
    type Environment,
    type FileSystem,
    FileSystemError,
} from "@destack/environment";
import { BunFileSystem } from "@destack/environment/bun";
import { Digest, present, schema } from "@destack/schema";
import type { TestMatrix } from "../project/index.ts";

/** The shared test options: one module graph per worker, cached modules, and quick polls. */
const TEST = { isolate: false, fsModuleCache: true } as const;

/** The test server options, whose dependencies the runner leaves to Bun or inlines. */
type TestServer = NonNullable<ViteUserConfig["test"]>["server"];

/** Add the person's caches to the modules a test server leaves to Bun to load. */
function withCaches(
    server: TestServer,
    environment: Environment<"process">,
): NonNullable<TestServer> {
    // match the person's cache directory and the one the environment names
    const { variables } = environment.process;
    const named = variables["DESTACK_CACHE_DIRECTORY"];
    const caches = [
        BunFileSystem.cacheDirectoryOf(variables),
        ...(named === undefined ? [] : [named]),
    ];
    const external = caches.map((cache) => new RegExp(`${RegExp.escape(cache)}/`, "u"));

    return {
        ...server,
        deps: { ...server?.deps, external: [...external, ...(server?.deps?.external ?? [])] },
    };
}

/** How often `expect.poll` rechecks in milliseconds, a local sync round trip taking a few. */
const POLL_INTERVAL = 5;

/** A test tier: how a test costs, which sets when it counts as slow and when as hung. */
export type TestTier = "unit" | "scenario" | "bench";

/** The milliseconds after which a tier's test is reported as slow, and fails as hung. */
export const TEST_TIERS = {
    /** One package's nouns in process: tens of milliseconds each, as AGENTS CT40 asks. */
    unit: { slow: 100, hang: 5_000 },
    /** A story across a simulated world, which boots in about a second, on CI about two. */
    scenario: { slow: 5_000, hang: 60_000 },
    /** A measurement, its world's setup included, reported by its own numbers and never as slow. */
    bench: { slow: 600_000, hang: 600_000 },
} as const satisfies Record<TestTier, { readonly slow: number; readonly hang: number }>;

/** The variable listing the tiers a run includes, comma separated, every tier when absent. */
export const TIERS_VARIABLE = "DESTACK_TEST_TIERS";

/** The test options a tier sets, which configurations leave to it. */
type TierOption = "testTimeout" | "hookTimeout" | "slowTestThreshold";

/** The test options of a tier: its hang ceiling for tests, hooks and polls, and slow threshold. */
function tierOptions(tier: TestTier) {
    const { slow, hang } = TEST_TIERS[tier];

    return {
        testTimeout: hang,
        hookTimeout: hang,
        slowTestThreshold: slow,
        expect: { poll: { interval: POLL_INTERVAL, timeout: hang } },
    };
}

/** The workspace patterns and exports of a package.json. */
const Manifest = schema.looseObject({
    workspaces: schema.array(schema.string()).exactOptional(),
    exports: schema.record(schema.string(), schema.unknown()).exactOptional(),
});

/** The build extension a destack.json declares, such as `./build#viewExtension`. */
const Definition = schema.looseObject({ build: schema.string().exactOptional() });

/** A test project, its tier and the targets it runs against. */
export type ProjectConfiguration = Omit<TestProjectInlineConfiguration, "test"> & {
    /** The tier of its tests. */
    readonly tier: TestTier;
    /** The values it runs against by kind, one project per combination. */
    readonly targets?: TestMatrix;
    /** The test options, the tier's own left to it. */
    readonly test?: Omit<NonNullable<TestProjectInlineConfiguration["test"]>, TierOption>;
};

/** The test options of a configuration, the tier's own left to it. */
type TestOptions = Omit<NonNullable<ViteUserConfig["test"]>, "projects" | TierOption>;

/** A test configuration: its tier alone, or its projects each with theirs. */
export type TestConfiguration = Omit<ViteUserConfig, "test"> & {
    /** The values its tests run against by kind, where its projects declare none. */
    readonly targets?: TestMatrix;
} & (
        | {
              /** The tier of its tests. */
              readonly tier: TestTier;
              /** The test options. */
              readonly test?: TestOptions;
          }
        | {
              /** The tier, which each project declares instead. */
              readonly tier?: never;
              /** The test options and the projects, each run once per combination. */
              readonly test: TestOptions & { readonly projects: readonly ProjectConfiguration[] };
          }
    );

/** Define a test configuration whose package sources receive Destack module metadata. */
export function defineConfiguration(
    configuration: TestConfiguration,
    environment: Environment<"files" | "process">,
): ViteUserConfig {
    // split off the tier, the targets, the projects and the global setup the projects run in the root's place
    const { tier, targets, test, ...rest } = configuration;
    const { globalSetup, ...options } = test ?? {};
    // keep the root's own options apart from the projects it lists, which the tiers filter
    const { projects: _listed, ...root } = { projects: undefined, ...options };
    const listed = test !== undefined && "projects" in test ? test.projects : undefined;
    const tiers = selectedTiers(environment);
    const declared = listed?.filter((project) => tiers === undefined || tiers.has(project.tier));
    const setup = globalSetup === undefined ? {} : { globalSetup };
    const kept = declared?.length === 0 ? undefined : declared;
    const isExcluded =
        (tier !== undefined && tiers !== undefined && !tiers.has(tier)) || declared?.length === 0;

    // build the projects of a run that includes the tier, its targets' projects too
    const projects = isExcluded
        ? undefined
        : configuredProjects(kept, tier, targets, setup)?.map((project) => ({
              ...project,
              test: { ...project.test, server: withCaches(project.test?.server, environment) },
          }));
    const tiered = tier === undefined ? {} : tierOptions(tier);

    return vitest.defineConfig({
        ...rest,
        test: {
            ...TEST,
            ...(projects === undefined
                ? {
                      ...root,
                      ...setup,
                      ...tiered,
                      ...(tier === undefined ? {} : { provide: { ...root.provide, tier } }),
                      // run nothing in a tier the run leaves out
                      ...(isExcluded ? { exclude: ["**"], passWithNoTests: true } : {}),
                  }
                : { ...options, projects }),
            server: withCaches(options.server, environment),
        },
        plugins: [
            modulePlugin(new ModuleTransformer(environment)),
            cachePlugin(environment),
            ...(rest.plugins ?? []).map(withoutServer),
        ],
    });
}

/** Play each exported scenario of a module as tests, through the drivers of some modules. */
export function scenarioPlugin(modules: readonly string[]): Plugin {
    return {
        name: "destack-scenarios",
        transform: {
            filter: { id: /\.scenario\.tsx?$/u },
            handler(code, id) {
                // import the module itself, the runner and the drivers' modules, then play its exports
                const self = JSON.stringify(`./${basename(id)}`);
                const imported = modules.map(
                    (module, index) =>
                        `import * as __DestackDrivers${index} from ${JSON.stringify(module)};`,
                );
                const types = modules
                    .map((_, index) => `...__DestackRunner.driversOf(__DestackDrivers${index})`)
                    .join(", ");
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
    tier: TestTier | undefined,
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
    // run the whole configuration per combination of its targets, in its tier
    else if (targets !== undefined && tier !== undefined) {
        return targetProjects({ extends: true, tier, test: setup }, targets);
    }
    // run the root alone
    else {
        return undefined;
    }
}

/** Read the tiers a run includes from the environment, every tier when it lists none. */
function selectedTiers(environment: Environment<"process">): ReadonlySet<TestTier> | undefined {
    // read the listed tiers, refusing an unknown one
    const listed = environment.process.variables[TIERS_VARIABLE];
    if (listed === undefined || listed === "") {
        return undefined;
    }
    const tiers = listed.split(",").map((each) => each.trim());
    const unknown = tiers.find((each) => !(each in TEST_TIERS));
    if (unknown !== undefined) {
        throw new TypeError(`${TIERS_VARIABLE} lists unknown tier '${unknown}'`);
    }

    return new Set(tiers.filter((each): each is TestTier => each in TEST_TIERS));
}

/** List a project once per combination of its targets, or the configuration's, named after them. */
function targetProjects(
    project: ProjectConfiguration,
    declared: TestMatrix | undefined,
): TestProjectInlineConfiguration[] {
    // set the tier's options, and return a project without targets as it is
    const { targets: local, tier, ...options } = project;
    const configuration = {
        ...options,
        test: {
            ...options.test,
            ...tierOptions(tier),
            provide: { ...options.test?.provide, tier },
        },
    };
    const targets = local ?? declared;
    if (targets === undefined) {
        return [configuration];
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
        const named = configuration.test?.name;
        const name =
            named === undefined
                ? values
                : typeof named === "object"
                  ? { ...named, label: `${named.label}:${values}` }
                  : `${named}:${values}`;

        // provide the values to its tests
        return {
            ...configuration,
            test: {
                ...configuration.test,
                name,
                provide: { ...configuration.test?.provide, targets: combination },
            },
        };
    });
}

/** Key cached module transforms by every package definition the module transform reads. */
function cachePlugin(environment: Environment<"files" | "process">): Plugin {
    let fingerprint: string | undefined;

    return {
        name: "destack-module-cache",
        async configResolved() {
            // digest the definitions once, before any transform asks for the key
            fingerprint = await definitions(environment);
        },
        configureVitest({ defineCacheKeyGenerator }) {
            defineCacheKeyGenerator(() => present(fingerprint, "the definitions' fingerprint"));
        },
    };
}

/** Digest the workspace's package definitions and the module transform's sources. */
async function definitions(environment: Environment<"files" | "process">): Promise<string> {
    // find the workspace root listing the packages
    const { files } = environment;
    const start = environment.process.workingDirectory;
    let root = start;
    let workspaces = await readWorkspaces(root, files);
    while (workspaces === undefined) {
        if (dirname(root) === root) {
            throw new TypeError(`no workspace above '${start}'`);
        }
        root = dirname(root);
        workspaces = await readWorkspaces(root, files);
    }

    // read each package's destack.json and package.json, skipping dependencies and hidden folders
    const packages: string[] = [];
    for (const member of await members(root, workspaces, files)) {
        const found = await walk(member, files);
        packages.push(
            ...found.filter((file) => ["destack.json", "package.json"].includes(basename(file))),
        );
    }

    // read the module transform's sources and its Vite plugin
    const transform = dirname(fileURLToPath(import.meta.resolve("@destack/package/transform")));
    const entries = await files.readDirectory(transform);
    const sources = [
        ...entries
            .filter((entry) => entry.kind === "file" && entry.name.endsWith(".ts"))
            .map((entry) => join(transform, entry.name)),
        fileURLToPath(import.meta.resolve("@destack/package/vite")),
    ];
    const extensions: string[] = [];
    for (const file of packages.filter((path) => basename(path) === "destack.json")) {
        extensions.push(...(await extensionSources(dirname(file), files)));
    }
    const digested = [...packages, ...sources, ...extensions].toSorted();

    // hash each file's path and content
    return Digest.of(fingerprinted(digested, files), environment.crypto);
}

/** Stream each file's path and content, each ending in a zero byte. */
async function* fingerprinted(
    paths: readonly string[],
    files: FileSystem,
): AsyncGenerator<Uint8Array> {
    const encoder = new TextEncoder();
    for (const path of paths) {
        yield encoder.encode(`${path}\0`);
        yield await files.readFile(path);
        yield encoder.encode("\0");
    }
}

/** Expand a workspace's member patterns, a `*` segment matching any directory. */
async function members(
    root: string,
    patterns: readonly string[],
    files: FileSystem,
): Promise<string[]> {
    const expanded: string[] = [];
    for (const pattern of patterns) {
        // expand the pattern segment by segment
        let directories = [root];
        for (const segment of pattern.split("/")) {
            const next: string[] = [];
            for (const parent of directories) {
                if (segment !== "*") {
                    next.push(join(parent, segment));
                } else {
                    const entries = await entriesOf(parent, files);
                    next.push(
                        ...entries
                            .filter((entry) => entry.kind === "directory")
                            .map((entry) => join(parent, entry.name)),
                    );
                }
            }
            directories = next;
        }
        expanded.push(...directories);
    }

    return expanded;
}

/** List the files below a directory, skipping dependencies and hidden folders. */
async function walk(directory: string, files: FileSystem): Promise<string[]> {
    // descend into the directories and take the files, finding none in a missing directory
    const found: string[] = [];
    for (const entry of await entriesOf(directory, files)) {
        const path = join(directory, entry.name);
        const isSkipped = entry.name === "node_modules" || entry.name.startsWith(".");
        if (entry.kind === "directory" && !isSkipped) {
            found.push(...(await walk(path, files)));
        } else if (entry.kind === "file") {
            found.push(path);
        }
    }

    return found;
}

/** List a directory's entries, none for a directory that does not exist. */
async function entriesOf(directory: string, files: FileSystem): Promise<DirectoryEntry[]> {
    return await files.readDirectory(directory).catch((error: unknown) => {
        // read a missing directory as empty
        if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
            return [];
        }
        throw error;
    });
}

/** List the sources of a package's build extension: the modules beside its build export. */
async function extensionSources(directory: string, files: FileSystem): Promise<string[]> {
    // read the build export the package's definition names, if it names one
    const definition = Definition.parse(
        JSON.parse(await files.readTextFile(join(directory, "destack.json"))),
    );
    if (definition.build === undefined) {
        return [];
    }
    const [subpath = ""] = definition.build.split("#", 1);
    const manifest = Manifest.parse(
        JSON.parse(await files.readTextFile(join(directory, "package.json"))),
    );
    const file = manifest.exports?.[subpath];
    if (typeof file !== "string") {
        throw new TypeError(`'${directory}' exports no build module at '${subpath}'`);
    }

    // list the modules of the export's directory, its tests aside
    const root = dirname(join(directory, file));
    const modules = await walk(root, files);

    return modules.filter((path) => /\.tsx?$/u.test(path) && !path.endsWith(".test.ts"));
}

/** Read the workspace patterns a directory's package.json lists, if it lists any. */
async function readWorkspaces(directory: string, files: FileSystem): Promise<string[] | undefined> {
    // read no workspaces from a directory without a package.json
    const text = await files
        .readTextFile(join(directory, "package.json"))
        .catch((error: unknown) => {
            if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                return undefined;
            }
            throw error;
        });

    return text === undefined ? undefined : Manifest.parse(JSON.parse(text)).workspaces;
}
