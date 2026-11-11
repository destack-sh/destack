import { basename, dirname, join, relative, sep } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import {
    DependencyName,
    PackageDefinition,
    PackageJson,
    type PackageRole,
    RuntimeRequirements,
    type DependencyPackage,
    DependencyRelease,
    type DependencyResolution,
} from "@destack/package";
import type { NpmDependency } from "./npm.ts";
import {
    type BuildReader,
    MANIFEST_PATH,
    PackageManifest,
    type PackageOutput,
} from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { HOST_RUNTIME, Runtime } from "@destack/package/runtime";
import { runtimeConditions } from "@destack/package/build";
import { Digest, present, schema } from "@destack/schema";
import { BunLockfile } from "@destack/check/bun";
import { locateWorkspace } from "@destack/check/workspace";
import { BuildError } from "../error/index.ts";
import type { LocalBuildStore } from "../store/local.ts";

/** The directory below the trees staging what a materialization writes before moving it in. */
export const STAGING_DIRECTORY = ".staging";

/** How a workspace installed npm packages: for its members, and where it put each one. */
interface WorkspaceInstall {
    /** The workspace's members, whose builds link the npm packages it installed. */
    readonly members: ReadonlySet<string>;
    /** Find where it installed an owner's npm package, absent for a missing optional one. */
    readonly find: (
        owner: string,
        name: string,
        release: DependencyPackage,
        isRequired: boolean,
    ) => Promise<string | undefined>;
}

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

/** The file a Destack package declares itself in, a source member and its builds alike. */
const DEFINITION_FILE = "destack.json";

/** The package.json a materialized dependency has. */
const MaterializedPackage = schema.looseObject({
    name: schema.string().min(1),
    exports: MaterializedExports,
});

/** A running tree's runtime and the builds it places, its outputs alone. */
export interface TreeRun {
    /** The runtime whose outputs the tree places. */
    readonly runtime: Runtime;
    /** The builds to place, a workload's closure. */
    readonly closure: ReadonlySet<string>;
}

/** A build a materialization visited: where its tree goes and the builds it links. */
interface TreeVisit {
    /** The build. */
    readonly dependency: DependencyBuild;
    /** The dependencies the build was compiled with, by name. */
    readonly recorded: Readonly<Record<string, DependencyRelease>>;
    /** The directory its tree goes in. */
    readonly root: string;
    /** The Destack builds it links that the tree places. */
    readonly linked: readonly DependencyBuild[];
}

/** The builds of a package's Destack dependencies, materialized as a node_modules tree. */
export class DependencyTree {
    /** The directory of each dependency's build or npm unit, by package name. */
    readonly directories: Readonly<Record<string, string>>;
    /** The role of each placed Destack build, by package name. */
    readonly roles: ReadonlyMap<string, PackageRole>;
    /** The exports of each Destack dependency, by package name. */
    readonly #exports: ReadonlyMap<string, schema.Infer<typeof MaterializedExports>>;

    /** Read a tree from its dependencies' directories, their roles and their exports. */
    private constructor(
        directories: Readonly<Record<string, string>>,
        roles: ReadonlyMap<string, PackageRole>,
        exports: ReadonlyMap<string, schema.Infer<typeof MaterializedExports>>,
    ) {
        this.directories = directories;
        this.roles = roles;
        this.#exports = exports;
    }

    /** Materialize a package's Destack dependencies in the store's trees, once per build. */
    static async install(
        dependencies: Readonly<Record<string, DependencyResolution>>,
        directory: string,
        store: LocalBuildStore,
    ): Promise<DependencyTree> {
        // write each build once in the store's trees, linking the npm packages the workspace installed
        const builds = Object.values(dependencies).filter(
            (dependency): dependency is DependencyBuild => dependency.kind === "destack",
        );
        const workspace = await DependencyTree.#workspace(directory, store.environment.files);

        return await DependencyTree.materialize(store, builds, store.trees, { workspace });
    }

    /** Write builds and theirs once each under a directory by manifest, with their npm packages. */
    static async materialize(
        store: LocalBuildStore,
        dependencies: readonly DependencyRelease[],
        directory: string,
        options: {
            /** The workspace whose Bun store links the npm packages, else the store places them. */
            readonly workspace?: string | undefined;
            /** A running tree's runtime and closure, placed as outputs alone, else whole builds. */
            readonly run?: TreeRun;
        } = {},
    ): Promise<DependencyTree> {
        // link npm packages to a workspace's Bun store, else place the npm units the builds record
        const { files } = store.environment;
        const { workspace, run } = options;
        const closure = run?.closure;
        const installation =
            workspace === undefined
                ? undefined
                : await DependencyTree.#installation(workspace, files);

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
                run,
            );
            placedNpm[unit.package.name] = placed;
        }

        // write each build once, following the Destack dependencies it was compiled with
        const directories: Record<string, string> = {};
        const builds = dependencies.filter(
            (each): each is DependencyBuild => each.kind === "destack",
        );
        const manifests = new Map<string, string>();
        const visits = new Map<string, TreeVisit>();
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
            const reader = await store.contents(dependency.manifest);
            const { manifest } = reader;
            const recorded: Record<string, DependencyRelease> = {};
            const outputs = Object.values(manifest.outputs).toSorted(
                (left, right) =>
                    Number(left.runtime === HOST_RUNTIME) - Number(right.runtime === HOST_RUNTIME),
            );
            // take each npm unit from the Bun output where it has one, which the tree places
            for (const output of outputs) {
                Object.assign(recorded, output.dependencies);
            }

            // place each builtin a compile reads at its locked build
            if (run === undefined) {
                const locked = new Map(
                    Object.values(await reader.dependencies()).flatMap((each) =>
                        each.kind === "destack" ? [[each.package.name, each] as const] : [],
                    ),
                );
                for (const [name, release] of Object.entries(recorded)) {
                    if (release.kind === "builtin") {
                        recorded[name] = present(
                            locked.get(name),
                            `the build of ${name} that ${manifest.package.name} compiled against`,
                        );
                    }
                }
            }
            const linked = Object.values(recorded).filter(
                (each): each is DependencyBuild =>
                    each.kind === "destack" && (closure?.has(each.manifest) ?? true),
            );
            pending.push(...linked);

            // record where the build goes and the builds it links
            const root = join(directory, DependencyTree.#rootName(dependency.manifest, run));
            directories[dependency.package.name] = join(
                root,
                "node_modules",
                dependency.package.name,
            );
            visits.set(dependency.manifest, { dependency, recorded, root, linked });
        }

        // place each build after the builds it links
        const roots = builds.map((each) => each.manifest);
        for (const visit of DependencyTree.#linkedFirst(visits, roots)) {
            // clone what the store still lacks of a tree the store's layer ships
            const shipped = store.layeredTree(basename(visit.root));
            if (shipped !== undefined) {
                await DependencyTree.#adopt(shipped, visit.root, files);
            }
            // place a tree outside the layer
            else if (!(await files.exists(visit.root))) {
                await DependencyTree.#place(store, visit.dependency, visit.recorded, directory, {
                    installation,
                    named,
                    run,
                });
            }
        }

        // read the builds' exports, beside the npm units placed directly
        const tree = await DependencyTree.read(directories, files);

        return new DependencyTree({ ...placedNpm, ...tree.directories }, tree.roles, tree.#exports);
    }

    /** Order visited builds from some roots so each follows the builds it links, each once. */
    static #linkedFirst(
        visits: ReadonlyMap<string, TreeVisit>,
        roots: readonly string[],
    ): TreeVisit[] {
        // walk depth first, taking a build once every build it links is taken
        const ordered: TreeVisit[] = [];
        const seen = new Set<string>();
        const pending = roots.toReversed().map((manifest) => ({ manifest, isExpanded: false }));
        for (let entry = pending.pop(); entry !== undefined; entry = pending.pop()) {
            // take an expanded build, else expand a first visit with its links on top
            const visit = visits.get(entry.manifest);
            if (visit !== undefined && entry.isExpanded) {
                ordered.push(visit);
            } else if (visit !== undefined && !seen.has(entry.manifest)) {
                seen.add(entry.manifest);
                const links = visit.linked.map((each) => ({
                    manifest: each.manifest,
                    isExpanded: false,
                }));
                pending.push({ manifest: entry.manifest, isExpanded: true }, ...links.toReversed());
            }
        }

        return ordered;
    }

    /** Write one build into its place below a directory: its files, links and package.json. */
    static async #place(
        store: LocalBuildStore,
        dependency: DependencyBuild,
        recorded: Readonly<Record<string, DependencyRelease>>,
        directory: string,
        workspace: {
            readonly installation: WorkspaceInstall | undefined;
            readonly named: ReadonlySet<string>;
            readonly run: TreeRun | undefined;
        },
    ): Promise<void> {
        // stage the build beside its place, linking as siblings the Destack dependencies the tree places
        const { files } = store.environment;
        const reader = await store.contents(dependency.manifest);
        const staged = await DependencyTree.#stage(directory, files);
        const root = join(directory, DependencyTree.#rootName(dependency.manifest, workspace.run));
        const target = join(staged, "node_modules", dependency.package.name);
        for (const [name, release] of Object.entries(recorded)) {
            if (
                release.kind === "destack" &&
                (workspace.run?.closure.has(release.manifest) ?? true)
            ) {
                const sibling = join(staged, "node_modules", name);
                const linkedRoot = DependencyTree.#rootName(release.manifest, workspace.run);
                const linked = join(directory, linkedRoot, "node_modules", name);
                await DependencyTree.#symlink(linked, sibling, { staged, root }, files);
            }
        }

        // clone the build's files but its package.json, which the tree writes anew, then write its manifest
        const built = await DependencyTree.#placed(reader, workspace.run);
        await store.place(
            built
                .filter((file) => file.path !== "package.json")
                .map((file) => ({ file, destination: join(target, file.path) })),
        );
        await files.writeFile(
            join(target, MANIFEST_PATH),
            PackageManifest.serialize(reader.manifest),
        );

        // link a member's npm packages from the workspace's Bun store, others' as their host output records
        const authored = AuthoredPackage.parse(
            JSON.parse(new TextDecoder().decode(await reader.load("package.json"))),
        );
        const { installation, named, run } = workspace;
        const hosted = await DependencyTree.#hosted(store, reader.manifest, dependency, run);
        const linked =
            installation === undefined || !installation.members.has(dependency.package.name)
                ? await DependencyTree.#placeRecorded(store, hosted, directory, run)
                : await DependencyTree.#installed(
                      installation,
                      dependency,
                      recorded,
                      authored,
                      named,
                  );
        for (const [name, placed] of linked) {
            const link = join(target, "node_modules", name);
            await DependencyTree.#symlink(placed, link, { staged, root }, files);
        }

        // export its outputs' modules and its authored sources' types from its package.json
        const outputs = Object.values(reader.manifest.outputs);
        const exports = DependencyTree.#exportsOf(outputs, authored.exports);
        const packageJson = { ...authored, type: "module", exports };
        await files.writeTextFile(join(target, "package.json"), JSON.stringify(packageJson));

        // move it into place, leaving the one a concurrent materialization moved in first
        await DependencyTree.#move(staged, root, files);
    }

    /** Read the pins a build's host output runs with: the run's closure, else its exports'. */
    static async #hosted(
        store: LocalBuildStore,
        manifest: PackageManifest,
        dependency: DependencyBuild,
        run: TreeRun | undefined,
    ): Promise<Readonly<Record<string, DependencyRelease>>> {
        const pinned = manifest.outputs[HOST_RUNTIME]?.dependencies ?? {};
        const imported =
            run?.closure ??
            (await store.closure(
                dependency.manifest,
                dependency.package.id,
                HOST_RUNTIME,
                undefined,
                "export",
            ));

        return Object.fromEntries(
            Object.entries(pinned).filter(
                ([, each]) => DependencyRelease.hasManifest(each) && imported.has(each.manifest),
            ),
        );
    }

    /** Place the npm units a build's outputs record, each beside the others as its peers. */
    static async #placeRecorded(
        store: LocalBuildStore,
        recorded: Readonly<Record<string, DependencyRelease>>,
        directory: string,
        run: TreeRun | undefined,
    ): Promise<Map<string, string>> {
        // place each recorded unit with the others beside it
        const units = Object.entries(recorded).filter(
            (entry): entry is [string, NpmDependency] =>
                entry[1].kind === "npm" && (run?.closure.has(entry[1].manifest) ?? true),
        );
        const beside = new Map(units);
        const placed = new Map<string, string>();
        for (const [name, unit] of units) {
            placed.set(
                name,
                await DependencyTree.#placeNpm(store, unit, directory, beside, new Set(), run),
            );
        }

        return placed;
    }

    /** Find the npm packages a build imports and declares as the workspace installed them. */
    static async #installed(
        installation: WorkspaceInstall,
        dependency: DependencyBuild,
        recorded: Readonly<Record<string, DependencyRelease>>,
        authored: schema.Infer<typeof AuthoredPackage>,
        named: ReadonlySet<string>,
    ): Promise<Map<string, string>> {
        // take the npm units its outputs record, at their locked versions
        const owner = dependency.package.name;
        const linked = new Map<string, string | undefined>();
        for (const [name, release] of Object.entries(recorded)) {
            if (release.kind === "npm") {
                linked.set(name, await installation.find(owner, name, release.package, true));
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
                const release = { name, version };
                linked.set(name, await installation.find(owner, name, release, isRequired));
            }
        }

        return new Map(
            [...linked].flatMap(([name, placed]) =>
                placed === undefined ? [] : [[name, placed] as const],
            ),
        );
    }

    /** Read how a workspace installs npm packages: by its members' links, else in its Bun store. */
    static async #installation(workspace: string, files: FileSystem): Promise<WorkspaceInstall> {
        // read the members of the workspace the lockfile belongs to, none outside a lockfile
        const located = await DependencyTree.#workspace(workspace, files);
        const lockfile = await BunLockfile.read(located, files);
        const members = lockfile === undefined ? {} : lockfile.workspaces;
        const installed = join(located, "node_modules", ".bun");
        const listed = (await files.exists(installed)) ? await files.readDirectory(installed) : [];
        const entries = listed.map((entry) => entry.name);
        const sources = new Map(
            Object.entries(members).flatMap(([path, member]) =>
                member.name === undefined ? [] : [[member.name, join(located, path)] as const],
            ),
        );

        // find a package as its owner's source installs it, else as the store has the release
        return {
            members: new Set(sources.keys()),
            find: async (owner, name, release, isRequired) =>
                (await DependencyTree.#linked(sources.get(owner), name, files)) ??
                DependencyTree.#installedIf(installed, entries, release, isRequired),
        };
    }

    /** Make a fresh staging directory beside the trees, so moving it into place is atomic. */
    static async #stage(directory: string, files: FileSystem): Promise<string> {
        const staging = join(directory, STAGING_DIRECTORY);
        await files.mkdir(staging, { recursive: true });

        return await files.makeTemporaryDirectory("tree-", { directory: staging });
    }

    /** Move a staged directory into place, leaving the one a concurrent writer moved in first. */
    static async #move(staged: string, root: string, files: FileSystem): Promise<void> {
        await files.rename(staged, root).catch(async (error: unknown) => {
            if (!(await files.exists(root))) {
                throw error;
            }
            await files.remove(staged, { recursive: true });
        });
    }

    /** Find where an npm unit is placed: by its manifest, and by its peers' places if any. */
    static async #npmRoot(
        store: LocalBuildStore,
        dependency: NpmDependency,
        directory: string,
        scope: ReadonlyMap<string, NpmDependency>,
        visiting: ReadonlySet<string>,
        run: TreeRun | undefined,
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
        const peers = new Map<string, NpmDependency>();
        const declaredPeers = Object.entries(output.peers)
            .filter(([peer]) => !Object.hasOwn(output.dependencies, peer))
            .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)));
        for (const [peer, declared] of declaredPeers) {
            // take a provided peer, and refuse a required one its dependent leaves out
            const provided = scope.get(peer);
            if (provided !== undefined) {
                peers.set(peer, provided);
            } else if (!declared.isOptional) {
                throw BuildError.of(
                    "DEPENDENCY_UNRESOLVED",
                    `${name} requires the peer ${peer}, which its dependent does not provide`,
                );
            }
        }

        // place a unit without peers by its manifest, and one with peers by its peers' places too
        if (peers.size === 0) {
            return {
                root: join(directory, DependencyTree.#rootName(dependency.manifest, run)),
                peers,
            };
        }
        const placed: Record<string, string> = {};
        for (const [peer, provided] of peers) {
            const next = new Set([...visiting, dependency.manifest]);
            placed[peer] = (
                await DependencyTree.#npmRoot(store, provided, directory, scope, next, run)
            ).root;
        }

        return {
            root: join(
                directory,
                DependencyTree.#rootName(
                    `${dependency.manifest}+${await Digest.json(placed, store.environment.crypto)}`,
                    run,
                ),
            ),
            peers,
        };
    }

    /** Place an npm unit's Bun output once, its dependencies' units and its peers beside it. */
    static async #placeNpm(
        store: LocalBuildStore,
        dependency: NpmDependency,
        directory: string,
        scope: ReadonlyMap<string, NpmDependency>,
        placing: ReadonlySet<string>,
        run: TreeRun | undefined,
    ): Promise<string> {
        // use a unit placed before or being placed, its peers taken from the packages beside it
        const name = dependency.package.name;
        const { root, peers } = await DependencyTree.#npmRoot(
            store,
            dependency,
            directory,
            scope,
            new Set(),
            run,
        );
        const { files } = store.environment;
        const target = join(root, "node_modules", name);
        const shipped = store.layeredTree(basename(root));
        // leave a tree this materialization places
        if (placing.has(root)) {
            return target;
        }
        // clone what the store still lacks of a tree the store's layer ships
        else if (shipped !== undefined) {
            await DependencyTree.#adopt(shipped, root, files);

            return target;
        }
        // leave a tree placed before
        else if (await files.exists(root)) {
            return target;
        }
        const reader = await store.contents(dependency.manifest);
        const output = present(
            reader.manifest.outputs[HOST_RUNTIME],
            `the ${HOST_RUNTIME} output of ${name}`,
        );
        const staged = await DependencyTree.#stage(directory, files);

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
            const placed = await DependencyTree.#placeNpm(
                store,
                release,
                directory,
                beside,
                next,
                run,
            );
            const sibling = join(staged, "node_modules", imported);
            await DependencyTree.#symlink(placed, sibling, { staged, root }, files);
        }

        // write its Bun output's files as the package
        const prefix = `${output.directory}/`;
        const placed = join(staged, "node_modules", name);
        await store.place(
            (await DependencyTree.#placed(reader, run))
                .filter((file) => file.path.startsWith(prefix))
                .map((file) => ({
                    file,
                    destination: join(placed, file.path.slice(prefix.length)),
                })),
        );

        // resolve its converted modules by the exports its build records, then move it into place
        const exports = Object.fromEntries(
            Object.entries(output.exports).map(([subpath, module]) => [
                subpath,
                `./${module.path.slice(prefix.length)}`,
            ]),
        );
        await files.mkdir(placed, { recursive: true });
        await files.writeTextFile(
            join(placed, "package.json"),
            JSON.stringify({ name, type: "module", exports }),
        );
        await DependencyTree.#move(staged, root, files);

        return target;
    }

    /** Link a staged path to a sibling tree relatively, so trees move whole, else absolutely. */
    static async #symlink(
        target: string,
        link: string,
        tree: { readonly staged: string; readonly root: string },
        files: FileSystem,
    ): Promise<void> {
        // point the link from its place in the moved tree when the target lies beside the tree
        const moved = join(tree.root, relative(tree.staged, link));
        const isSibling = relative(dirname(tree.root), target).split(sep)[0] !== "..";
        await files.mkdir(dirname(link), { recursive: true });
        await files.symlink(isSibling ? relative(dirname(moved), target) : target, link);
    }

    /** List a build's files a tree places: a running tree's output alone, else the whole build. */
    static async #placed(
        reader: BuildReader,
        run: TreeRun | undefined,
    ): Promise<readonly PackageFile[]> {
        // place every file for a compile
        if (run === undefined) {
            return await reader.distributed();
        }

        // place the output a running tree runs, with its maps
        const [name] = present(
            Object.entries(reader.manifest.outputs).find(
                ([, output]) => output.runtime === run.runtime,
            ),
            `the ${run.runtime} output of ${reader.manifest.package.name}`,
        );

        return await reader.outputFiles(name);
    }

    /** Read the build a tree root holds by its name, absent for a name of no root. */
    static manifestOf(name: string): Digest | undefined {
        const [manifest] = name.split(/[+.]/u);
        const parsed = Digest.safeParse(manifest);

        return parsed.success ? parsed.data : undefined;
    }

    /** Name a tree root: a build's manifest, and the runtime for a running tree's outputs alone. */
    static #rootName(base: string, run: TreeRun | undefined): string {
        return run === undefined ? base : `${base}.${run.runtime}`;
    }

    /** Clone every tree a layer's tree reaches that the store lacks, whole once it returns. */
    static async #adopt(shipped: string, root: string, files: FileSystem): Promise<void> {
        // leave a root in place, which has every tree it reaches in place
        if (await files.exists(root)) {
            return;
        }

        // clone each reached tree missing beside the root, one a concurrent clone has not moved in too
        const reached = await DependencyTree.#follow([await files.realPath(shipped)], files);
        // clone the root last, so a root in place has every tree it reaches in place
        for (const tree of [...reached].toReversed()) {
            const owned = join(dirname(root), basename(tree));
            // clone a tree the store does not own yet
            if (!(await files.exists(owned))) {
                await DependencyTree.#clone(tree, owned, files);
            }
        }
    }

    /** Clone a tree to a root in a staging directory beside it, keeping a concurrent clone. */
    static async #clone(tree: string, root: string, files: FileSystem): Promise<void> {
        // clone the tree in a staging directory beside the root, which a replaced layer leaves intact
        await files.mkdir(dirname(root), { recursive: true });
        const staging = await files.makeTemporaryDirectory(`${basename(root)}.`, {
            directory: dirname(root),
        });
        const staged = join(staging, "tree");
        try {
            // move it into place, keeping the root a concurrent materialization placed first
            await files.copy(tree, staged);
            await files.rename(staged, root).catch((error: unknown) => {
                if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
                    throw error;
                }
            });
        } finally {
            await files.remove(staging, { recursive: true, force: true });
        }
    }

    /** Locate a directory's workspace where its lockfile really is, past a linked lockfile. */
    static async #workspace(directory: string, files: FileSystem): Promise<string> {
        const located = await locateWorkspace(directory, files);
        const lockfile = join(located, "bun.lock");

        return (await files.exists(lockfile)) ? dirname(await files.realPath(lockfile)) : located;
    }

    /** Link every package of the tree into a node_modules directory, as an output resolves them. */
    async link(directory: string, files: FileSystem): Promise<void> {
        // create the directory, empty for a tree of builtins alone, then link each package
        await files.mkdir(directory, { recursive: true });
        for (const [name, target] of Object.entries(this.directories)) {
            const link = join(directory, name);
            await files.mkdir(dirname(link), { recursive: true });
            await files.symlink(target, link);
        }
    }

    /** Read the tree of materialized dependency directories, by package name. */
    static async read(
        directories: Readonly<Record<string, string>>,
        files: FileSystem,
    ): Promise<DependencyTree> {
        // read each package's exports, and its role where it declares one
        const roles = new Map<string, PackageRole>();
        const exports = new Map<string, schema.Infer<typeof MaterializedExports>>();
        for (const [name, directory] of Object.entries(directories)) {
            // read its exports
            const packageJson = MaterializedPackage.parse(
                JSON.parse(await files.readTextFile(join(directory, "package.json"))),
            );
            exports.set(name, packageJson.exports);

            // read its declared role, absent for an npm package
            const definition = join(directory, DEFINITION_FILE);
            if (await files.exists(definition)) {
                roles.set(name, PackageDefinition.read(await files.readTextFile(definition)).role);
            }
        }

        return new DependencyTree(directories, roles, exports);
    }

    /** List the real roots a tree's packages link to, each once. */
    async roots(files: FileSystem): Promise<readonly string[]> {
        // start at each package's root
        const pending = await Promise.all(
            Object.values(this.directories).map((directory) =>
                files.realPath(DependencyTree.#rootOf(directory)),
            ),
        );

        return await DependencyTree.#follow(pending, files);
    }

    /** List the real roots a directory's node_modules links to, each once. */
    static async linkedRoots(directory: string, files: FileSystem): Promise<readonly string[]> {
        const linked = await DependencyTree.#linkedPackages(join(directory, "node_modules"), files);
        const pending = await Promise.all(
            linked.map(async (path) =>
                files.realPath(DependencyTree.#rootOf(await files.realPath(path))),
            ),
        );

        return await DependencyTree.#follow(pending, files);
    }

    /** Follow roots' linked packages to their roots, visiting each root once. */
    static async #follow(pending: string[], files: FileSystem): Promise<readonly string[]> {
        const roots = new Set<string>();
        for (let root = pending.pop(); root !== undefined; root = pending.pop()) {
            if (!roots.has(root)) {
                // take the links beside the root's packages and inside each package it places
                roots.add(root);
                const modules = join(root, "node_modules");
                const placed = await DependencyTree.#placedPackages(modules, files);
                const directories = [modules, ...placed.map((path) => join(path, "node_modules"))];
                for (const directory of directories) {
                    for (const linked of await DependencyTree.#linkedPackages(directory, files)) {
                        pending.push(
                            await files.realPath(
                                DependencyTree.#rootOf(await files.realPath(linked)),
                            ),
                        );
                    }
                }
            }
        }

        return [...roots].toSorted();
    }

    /** Find the root of a package placed below its node_modules directory. */
    static #rootOf(directory: string): string {
        return directory.slice(0, directory.lastIndexOf(`${sep}node_modules${sep}`));
    }

    /** List the package directories placed below a node_modules directory, scoped ones included. */
    static async #placedPackages(directory: string, files: FileSystem): Promise<string[]> {
        // read the directory's package directories, and those inside each scope
        const entries = await files.readDirectory(directory).catch(FileSystemError.absent);
        const placed: string[] = [];
        for (const entry of entries ?? []) {
            const path = join(directory, entry.name);
            // list a scope's packages
            if (entry.kind === "directory" && entry.name.startsWith("@")) {
                placed.push(...(await DependencyTree.#placedPackages(path, files)));
            }
            // list a package, leaving dotfiles such as .bin
            else if (entry.kind === "directory" && !entry.name.startsWith(".")) {
                placed.push(path);
            }
        }

        return placed;
    }

    /** List the linked package directories below a node_modules directory, scoped ones included. */
    static async #linkedPackages(directory: string, files: FileSystem): Promise<string[]> {
        // read the directory's links, and the links inside each scope
        const entries = await files.readDirectory(directory).catch(FileSystemError.absent);
        const linked: string[] = [];
        for (const entry of entries ?? []) {
            const path = join(directory, entry.name);
            if (entry.kind === "symlink") {
                linked.push(path);
            } else if (entry.kind === "directory" && entry.name.startsWith("@")) {
                linked.push(...(await DependencyTree.#linkedPackages(path, files)));
            }
        }

        return linked;
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
                    conditions[condition] = `./${module.path}`;
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
    static async #linked(
        source: string | undefined,
        name: string,
        files: FileSystem,
    ): Promise<string | undefined> {
        // read nothing for an owner outside the workspace's sources
        if (source === undefined) {
            return undefined;
        }

        // resolve the link, absent for a package the source does not install
        return await files.realPath(join(source, "node_modules", name)).catch((error: unknown) => {
            if (FileSystemError.is(error, "NOT_FOUND")) {
                return undefined;
            }
            throw error;
        });
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
