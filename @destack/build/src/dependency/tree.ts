import {
    exists,
    mkdir,
    mkdtemp,
    readFile,
    realpath,
    rename,
    rm,
    symlink,
    writeFile,
} from "node:fs/promises";
import { existsSync, readdirSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import {
    DependencyName,
    PackageJson,
    RuntimeRequirements,
    type DependencyPackage,
    type DependencyRelease,
    type DependencyResolution,
} from "@destack/package";
import type { NpmDependency } from "./npm.ts";
import { MANIFEST_PATH, type PackageOutput } from "@destack/package/manifest";
import { HOST_RUNTIME, Runtime } from "@destack/package/runtime";
import { runtimeConditions } from "@destack/package/build";
import { Digest, present, schema } from "@destack/schema";
import { BunLockfile } from "@destack/check/bun";
import { locateWorkspace } from "@destack/check/workspace";
import { BuildError } from "../error/index.ts";
import type { LocalPackageStore } from "../store/local.ts";

/** The directory below the trees staging what a materialization writes before moving it in. */
export const STAGING_DIRECTORY = ".staging";

/** Find where a workspace installs an owner's npm package, absent for a missing optional one. */
type NpmInstalled = (
    owner: string,
    name: string,
    release: DependencyPackage,
    isRequired: boolean,
) => string | undefined;

/** A Destack dependency taken by its build. */
export type DependencyBuild = Extract<DependencyRelease, { readonly kind: "destack" }>;

/** The exports of a materialized dependency: each subpath's declarations and modules by runtime. */
const MaterializedExports = schema.record(
    schema.string().min(1),
    schema.record(schema.string().min(1), schema.string().min(1)),
);

/** The package.json a build ships, its exports pointing at authored sources. */
const AuthoredPackage = RuntimeRequirements.extend({
    exports: schema.record(
        schema.string().min(1),
        schema.union([schema.string().min(1), schema.object({ types: schema.string().min(1) })]),
    ),
});

/** The package.json a materialized dependency has. */
const MaterializedPackage = schema.looseObject({
    name: schema.string().min(1),
    exports: MaterializedExports,
});

/** The builds of a package's Destack dependencies, materialized as a node_modules tree. */
export class DependencyTree {
    /** The directory of each dependency's build or npm unit, by package name. */
    readonly directories: Readonly<Record<string, string>>;
    /** The exports of each Destack dependency, by package name. */
    readonly #exports: ReadonlyMap<string, schema.Infer<typeof MaterializedExports>>;

    /** Read a tree from its dependencies' directories and their exports. */
    private constructor(
        directories: Readonly<Record<string, string>>,
        exports: ReadonlyMap<string, schema.Infer<typeof MaterializedExports>>,
    ) {
        this.directories = directories;
        this.#exports = exports;
    }

    /** Materialize a package's Destack dependencies in the store's trees, once per build. */
    static async install(
        dependencies: Readonly<Record<string, DependencyResolution>>,
        directory: string,
        store: LocalPackageStore,
    ): Promise<DependencyTree> {
        // write each build once in the store's trees, linking the npm packages the workspace installed
        const builds = Object.values(dependencies).filter(
            (dependency): dependency is DependencyBuild => dependency.kind === "destack",
        );
        const workspace = await DependencyTree.#workspace(directory);

        return await DependencyTree.materialize(store, builds, store.trees, workspace);
    }

    /** Write builds and theirs once each under a directory by manifest, with their npm packages. */
    static async materialize(
        store: LocalPackageStore,
        dependencies: readonly DependencyRelease[],
        directory: string,
        workspace?: string,
    ): Promise<DependencyTree> {
        // link npm packages to a workspace's Bun store, else place the npm units the builds record
        const installation =
            workspace === undefined ? undefined : await DependencyTree.#installation(workspace);

        // place the npm units given directly, each other's peers among them
        const placedNpm: Record<string, string> = {};
        const given = dependencies.filter((each): each is NpmDependency => each.kind === "npm");
        const placedBeside = new Map(given.map((each) => [each.package.name, each]));
        for (const unit of given) {
            const placed = await DependencyTree.#placeNpm(
                store,
                unit,
                directory,
                placedBeside,
                new Set(),
            );
            placedNpm[unit.package.name] = placed;
        }

        // write each build once, following the Destack dependencies it was compiled with
        const directories: Record<string, string> = {};
        const builds = dependencies.filter(
            (each): each is DependencyBuild => each.kind === "destack",
        );
        const manifests = new Map<string, string>();
        const named = new Set(builds.map((dependency) => dependency.package.name));
        const pending = [...builds];
        for (let dependency = pending.pop(); dependency !== undefined; dependency = pending.pop()) {
            // visit each package once, refusing two builds of one package in the flat tree
            const visited = manifests.get(dependency.package.name);
            if (visited !== undefined && visited !== dependency.manifest) {
                throw BuildError.of(
                    "DEPENDENCY_INVALID",
                    `${dependency.package.name} is required as two different builds: ${visited} and ${dependency.manifest}`,
                );
            } else if (visited !== undefined) {
                continue;
            }
            manifests.set(dependency.package.name, dependency.manifest);
            // visit the Destack dependencies the build's outputs import, as builds
            const { manifest } = await store.contents(dependency.manifest);
            const recorded: Record<string, DependencyRelease> = {};
            for (const output of Object.values(manifest.outputs)) {
                Object.assign(recorded, output.dependencies);
            }
            pending.push(
                ...Object.values(recorded).filter(
                    (each): each is DependencyBuild => each.kind === "destack",
                ),
            );

            // use a build an earlier materialization moved into place, else place it
            const root = join(directory, dependency.manifest);
            directories[dependency.package.name] = join(
                root,
                "node_modules",
                dependency.package.name,
            );
            if (!(await exists(root))) {
                await DependencyTree.#place(store, dependency, recorded, directory, {
                    installation,
                    named,
                });
            }
        }

        // read the builds' exports, beside the npm units placed directly
        const tree = await DependencyTree.read(directories);

        return new DependencyTree({ ...placedNpm, ...tree.directories }, tree.#exports);
    }

    /** Write one build into its place below a directory: its files, links and package.json. */
    static async #place(
        store: LocalPackageStore,
        dependency: DependencyBuild,
        recorded: Readonly<Record<string, DependencyRelease>>,
        directory: string,
        workspace: {
            readonly installation: NpmInstalled | undefined;
            readonly named: ReadonlySet<string>;
        },
    ): Promise<void> {
        // stage the build beside its place, linking its Destack dependencies as siblings
        const reader = await store.contents(dependency.manifest);
        const staged = await DependencyTree.#stage(directory);
        const target = join(staged, "node_modules", dependency.package.name);
        for (const [name, release] of Object.entries(recorded)) {
            if (release.kind === "destack") {
                const sibling = join(staged, "node_modules", name);
                await mkdir(dirname(sibling), { recursive: true });
                await symlink(
                    join(directory, release.manifest, "node_modules", name),
                    sibling,
                    "junction",
                );
            }
        }

        // clone each of the build's files, then write its manifest
        for (const file of await reader.distributed()) {
            const destination = join(target, file.path);
            await store.place(file, destination);
        }
        await writeFile(join(target, MANIFEST_PATH), JSON.stringify(reader.manifest));

        // link its npm packages: from the workspace's Bun store, else as the units its outputs record
        const authored = AuthoredPackage.parse(
            JSON.parse(new TextDecoder().decode(await reader.load("package.json"))),
        );
        const { installation, named } = workspace;
        const linked =
            installation === undefined
                ? await DependencyTree.#placeRecorded(store, recorded, directory)
                : DependencyTree.#installed(installation, dependency, recorded, authored, named);
        for (const [name, placed] of linked) {
            const link = join(target, "node_modules", name);
            await mkdir(dirname(link), { recursive: true });
            await symlink(placed, link, "junction");
        }

        // export its outputs' modules and its authored sources' types from its package.json
        const outputs = Object.values(reader.manifest.outputs);
        const exports = DependencyTree.#exportsOf(outputs, authored.exports);
        const packageJson = { ...authored, type: "module", exports };
        await writeFile(join(target, "package.json"), JSON.stringify(packageJson));

        // move it into place, leaving the one a concurrent materialization moved in first
        const root = join(directory, dependency.manifest);
        await rename(staged, root).catch(async (error: unknown) => {
            if (!(await exists(root))) {
                throw error;
            }
            await rm(staged, { recursive: true });
        });
    }

    /** Place the npm units a build's outputs record, each beside the others as its peers. */
    static async #placeRecorded(
        store: LocalPackageStore,
        recorded: Readonly<Record<string, DependencyRelease>>,
        directory: string,
    ): Promise<Map<string, string>> {
        // place each recorded unit with the others beside it
        const units = Object.entries(recorded).filter(
            (entry): entry is [string, NpmDependency] => entry[1].kind === "npm",
        );
        const beside = new Map(units);
        const placed = new Map<string, string>();
        for (const [name, unit] of units) {
            placed.set(
                name,
                await DependencyTree.#placeNpm(store, unit, directory, beside, new Set()),
            );
        }

        return placed;
    }

    /** Find the npm packages a build imports and declares as the workspace installed them. */
    static #installed(
        installation: NpmInstalled,
        dependency: DependencyBuild,
        recorded: Readonly<Record<string, DependencyRelease>>,
        authored: schema.Infer<typeof AuthoredPackage>,
        named: ReadonlySet<string>,
    ): Map<string, string> {
        // take the npm units its outputs record, at their locked versions
        const owner = dependency.package.name;
        const linked = new Map<string, string | undefined>();
        for (const [name, release] of Object.entries(recorded)) {
            if (release.kind === "npm") {
                linked.set(name, installation(owner, name, release.package, true));
            }
        }

        // take the npm packages its sources declare at a version, leaving its Destack ones to the tree
        for (const name of PackageJson.runtimeDependencies(authored)) {
            const version =
                authored.dependencies?.[name] ??
                authored.optionalDependencies?.[name] ??
                authored.peerDependencies?.[name];
            const isDestack = recorded[name]?.kind === "destack" || named.has(name);
            const isLinked = linked.has(name) || isDestack;
            // leave a linked or Destack package, and a peer without a version
            if (isLinked || version === undefined || version.startsWith("workspace:")) {
                continue;
            }
            // find the npm package as the workspace installed it
            else {
                const isRequired = !PackageJson.isOptional(authored, name);
                linked.set(name, installation(owner, name, { name, version }, isRequired));
            }
        }

        return new Map(
            [...linked].flatMap(([name, placed]) =>
                placed === undefined ? [] : [[name, placed] as const],
            ),
        );
    }

    /** Read how a workspace installs npm packages: by its members' links, else in its Bun store. */
    static async #installation(workspace: string): Promise<NpmInstalled> {
        // read the members of the workspace the lockfile belongs to, none outside a lockfile
        const located = await DependencyTree.#workspace(workspace);
        const lockfile = await BunLockfile.read(located);
        const members = lockfile === undefined ? {} : lockfile.workspaces;
        const installed = join(located, "node_modules", ".bun");
        const entries = existsSync(installed) ? readdirSync(installed) : [];
        const sources = new Map(
            Object.entries(members).flatMap(([path, member]) =>
                member.name === undefined ? [] : [[member.name, join(located, path)] as const],
            ),
        );

        // find a package as its owner's source installs it, else as the store has the release
        return (owner, name, release, isRequired) =>
            DependencyTree.#linked(sources.get(owner), name) ??
            DependencyTree.#installedIf(installed, entries, release, isRequired);
    }

    /** Make a fresh staging directory beside the trees, so moving it into place is atomic. */
    static async #stage(directory: string): Promise<string> {
        const staging = join(directory, STAGING_DIRECTORY);
        await mkdir(staging, { recursive: true });

        return await mkdtemp(join(staging, "tree-"));
    }

    /** Find where an npm unit is placed: by its manifest, and by its peers' places if any. */
    static async #npmRoot(
        store: LocalPackageStore,
        dependency: NpmDependency,
        directory: string,
        scope: ReadonlyMap<string, NpmDependency>,
        visiting: ReadonlySet<string>,
    ): Promise<{ readonly root: string; readonly peers: ReadonlyMap<string, NpmDependency> }> {
        // refuse peers requiring each other in a cycle
        const name = dependency.package.name;
        if (visiting.has(dependency.manifest)) {
            throw BuildError.of("DEPENDENCY_INVALID", `npm peer cycle through ${name}`);
        }

        // provide each peer the unit declares from the packages beside it, as Node resolves them
        const reader = await store.contents(dependency.manifest);
        const output = present(
            reader.manifest.outputs[HOST_RUNTIME],
            `the ${HOST_RUNTIME} output of ${name}`,
        );
        const declared = RuntimeRequirements.parse(
            JSON.parse(
                new TextDecoder().decode(await reader.load(`${output.directory}/package.json`)),
            ),
        );
        const names = PackageJson.peers(declared).filter(
            (each) => !Object.hasOwn(output.dependencies, each),
        );
        const peers = new Map(
            names.toSorted().flatMap((peer) => {
                const provided = scope.get(peer);

                return provided === undefined ? [] : [[peer, provided] as const];
            }),
        );

        // place a unit without peers by its manifest, and one with peers by its peers' places too
        if (peers.size === 0) {
            return { root: join(directory, dependency.manifest), peers };
        }
        const placed: Record<string, string> = {};
        for (const [peer, provided] of peers) {
            const next = new Set([...visiting, dependency.manifest]);
            placed[peer] = (
                await DependencyTree.#npmRoot(store, provided, directory, scope, next)
            ).root;
        }

        return {
            root: join(directory, `${dependency.manifest}+${await Digest.json(placed)}`),
            peers,
        };
    }

    /** Place an npm unit's Bun output once, its dependencies' units and its peers beside it. */
    static async #placeNpm(
        store: LocalPackageStore,
        dependency: NpmDependency,
        directory: string,
        scope: ReadonlyMap<string, NpmDependency>,
        placing: ReadonlySet<string>,
    ): Promise<string> {
        // use a unit placed before or being placed, its peers taken from the packages beside it
        const name = dependency.package.name;
        const { root, peers } = await DependencyTree.#npmRoot(
            store,
            dependency,
            directory,
            scope,
            new Set(),
        );
        const target = join(root, "node_modules", name);
        if (placing.has(root) || (await exists(root))) {
            return target;
        }
        const reader = await store.contents(dependency.manifest);
        const output = present(
            reader.manifest.outputs[HOST_RUNTIME],
            `the ${HOST_RUNTIME} output of ${name}`,
        );
        const staged = await DependencyTree.#stage(directory);

        // link the dependencies it records and its peers beside it
        const siblings = new Map<string, DependencyRelease>([
            ...Object.entries(output.dependencies),
            ...peers,
        ]);
        const beside = new Map<string, NpmDependency>([[name, dependency]]);
        for (const [imported, release] of siblings) {
            if (release.kind !== "npm") {
                throw BuildError.of(
                    "DEPENDENCY_INVALID",
                    `the npm unit of ${name} lists ${imported} as a build, not an npm release`,
                );
            }
            beside.set(imported, release);
        }
        for (const [imported, release] of beside) {
            if (imported === name) {
                continue;
            }
            const next = new Set([...placing, root]);
            const placed = await DependencyTree.#placeNpm(store, release, directory, beside, next);
            const sibling = join(staged, "node_modules", imported);
            await mkdir(dirname(sibling), { recursive: true });
            await symlink(placed, sibling, "junction");
        }

        // write its Bun output's files as the package, then move it into place
        const prefix = `${output.directory}/`;
        for (const file of await reader.distributed()) {
            if (file.path.startsWith(prefix)) {
                const destination = join(
                    staged,
                    "node_modules",
                    name,
                    file.path.slice(prefix.length),
                );
                await store.place(file, destination);
            }
        }
        await rename(staged, root).catch(async (error: unknown) => {
            if (!(await exists(root))) {
                throw error;
            }
            await rm(staged, { recursive: true });
        });

        return target;
    }

    /** Locate a directory's workspace where its lockfile really is, past a linked lockfile. */
    static async #workspace(directory: string): Promise<string> {
        const located = await locateWorkspace(directory);
        const lockfile = join(located, "bun.lock");

        return (await exists(lockfile)) ? dirname(await realpath(lockfile)) : located;
    }

    /** Link every package of the tree into a node_modules directory, as an output resolves them. */
    async link(directory: string): Promise<void> {
        for (const [name, target] of Object.entries(this.directories)) {
            const link = join(directory, name);
            await mkdir(dirname(link), { recursive: true });
            await symlink(target, link, "junction");
        }
    }

    /** Read the tree of materialized dependency directories, by package name. */
    static async read(directories: Readonly<Record<string, string>>): Promise<DependencyTree> {
        const exports = new Map<string, schema.Infer<typeof MaterializedExports>>();
        for (const [name, directory] of Object.entries(directories)) {
            const packageJson = MaterializedPackage.parse(
                JSON.parse(await readFile(join(directory, "package.json"), "utf8")),
            );
            exports.set(name, packageJson.exports);
        }

        return new DependencyTree(directories, exports);
    }

    /** List the declaration file of each dependency specifier, as TypeScript paths. */
    paths(): Record<string, string[]> {
        const paths: Record<string, string[]> = {};
        for (const [name, exports] of this.#exports) {
            for (const [subpath, conditions] of Object.entries(exports)) {
                const types = conditions["types"];
                if (types !== undefined) {
                    paths[DependencyTree.#specifier(name, subpath)] = [
                        join(DependencyTree.#directory(this.directories, name), types),
                    ];
                }
            }
        }

        return paths;
    }

    /** Find the export a specifier names among the Destack dependencies, by its runtime modules. */
    export(specifier: string): Readonly<Partial<Record<Runtime, string>>> | undefined {
        // find the package by its name, and the subpath past it
        const name = DependencyName.of(specifier);
        const exports = this.#exports.get(name);
        if (exports === undefined) {
            return undefined;
        }
        const subpath = specifier === name ? "." : `.${specifier.slice(name.length)}`;

        // refuse a subpath the package does not export
        const conditions = exports[subpath];
        if (conditions === undefined) {
            throw BuildError.of("DEPENDENCY_UNRESOLVED", `${name} exports no ${subpath}`);
        }

        // answer each runtime's module of the subpath, none for a subpath of types alone
        const directory = DependencyTree.#directory(this.directories, name);
        const modules: Partial<Record<Runtime, string>> = {};
        for (const runtime of Runtime.options) {
            const module = conditions[runtime];
            if (module !== undefined) {
                modules[runtime] = join(directory, module);
            }
        }

        return modules;
    }

    /** Map each export to its authored source's types and each output runtime's module. */
    static #exportsOf(
        outputs: readonly PackageOutput[],
        authored: schema.Infer<typeof AuthoredPackage>["exports"],
    ): schema.Infer<typeof MaterializedExports> {
        // type each export by its authored source, or the declarations a types-only export names
        const exports: Record<string, Record<string, string>> = {};
        for (const [subpath, source] of Object.entries(authored)) {
            exports[subpath] = { types: typeof source === "string" ? source : source.types };
        }

        // add each output's subpath modules under its runtime's conditions, the browser's last
        const ordered = outputs.toSorted(
            (left, right) =>
                Number(left.runtime === "browser") - Number(right.runtime === "browser"),
        );
        for (const output of ordered) {
            const subpaths = Object.entries(output.exports).filter(([name]) =>
                name.startsWith("."),
            );
            for (const [subpath, module] of subpaths) {
                const conditions = (exports[subpath] ??= {});
                for (const condition of DependencyTree.#conditions(output.runtime)) {
                    conditions[condition] = `./${module}`;
                }
            }
        }

        return exports;
    }

    /** List the conditions a runtime alone resolves, such as Bun's `bun` and `node`. */
    static #conditions(runtime: Runtime): string[] {
        const others = new Set(
            Runtime.options
                .filter((other) => other !== runtime)
                .flatMap((other) => runtimeConditions(other)),
        );

        return runtimeConditions(runtime).filter(
            (condition) => condition === runtime || !others.has(condition),
        );
    }

    /** Read the npm package an installed source links under a name, as its install resolved it. */
    static #linked(source: string | undefined, name: string): string | undefined {
        const link = source === undefined ? undefined : join(source, "node_modules", name);

        return link !== undefined && existsSync(link) ? realpathSync(link) : undefined;
    }

    /** Find an npm release among a Bun store's entries, failing for an absent required one. */
    static #installedIf(
        installed: string,
        listed: readonly string[],
        release: { readonly name: string; readonly version: string },
        isRequired: boolean,
    ): string | undefined {
        // match the release's store entry, which Bun suffixes with a hash of its peers
        const { name, version } = release;
        const entry = `${name.replace("/", "+")}@${version}`;
        const entries = listed.filter((each) => each === entry || each.startsWith(`${entry}+`));
        const [only, ...others] = entries;
        if (only === undefined && !isRequired) {
            return undefined;
        } else if (only === undefined) {
            throw BuildError.of(
                "DEPENDENCY_UNRESOLVED",
                `npm package not installed: ${name}@${version}`,
            );
        } else if (others.length > 0) {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `npm package installed with several peer sets: ${entries.join(", ")}`,
            );
        }

        return join(installed, only, "node_modules", name);
    }

    /** Write a package's specifier for one of its export subpaths. */
    static #specifier(name: string, subpath: string): string {
        return subpath === "." ? name : `${name}/${subpath.slice(2)}`;
    }

    /** Read a dependency's directory, which the tree's exports always have. */
    static #directory(directories: Readonly<Record<string, string>>, name: string): string {
        const directory = directories[name];
        if (directory === undefined) {
            throw BuildError.of(
                "INVARIANT_VIOLATED",
                `the dependency tree has no directory for ${name}`,
            );
        }

        return directory;
    }
}
