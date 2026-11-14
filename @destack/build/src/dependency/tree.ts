import { dirname, join, relative, sep } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import {
    DependencyName,
    PackageDefinition,
    PackageJson,
    type PackageRole,
    RuntimeRequirements,
    type DependencyPackage,
    DependencyRelease,
} from "@destack/package";
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

/** What a tree places: whole builds for a compile, or a running closure's outputs. */
export type TreeScope =
    | {
          /** A compile's tree, placing whole builds. */
          readonly kind: "compile";
      }
    | {
          /** A running tree. */
          readonly kind: "run";
          /** The runtime whose outputs the tree places. */
          readonly runtime: Runtime;
          /** The builds to place, a workload's closure. */
          readonly closure: ReadonlySet<string>;
      };

/** A build a materialization visited: where its tree goes and the packages it links. */
interface TreeVisit {
    /** The package's name. */
    readonly name: string;
    /** The build. */
    readonly entry: DependencyRelease;
    /** The build's manifest and files. */
    readonly reader: BuildReader;
    /** The directory its tree goes in. */
    readonly root: string;
    /** The packages it requires at run time, placed by the tree or not. */
    readonly required: readonly string[];
    /** The build each required package the tree places resolves to, by name. */
    readonly links: Readonly<Record<string, Digest>>;
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

    /** Materialize the builds a package compiles against in the store's trees, once each. */
    static async install(
        dependencies: Readonly<Record<string, DependencyRelease>>,
        links: Readonly<Record<Digest, Readonly<Record<string, Digest>>>>,
        directory: string,
        store: LocalBuildStore,
    ): Promise<DependencyTree> {
        // find the workspace the tree serves
        const workspace = await DependencyTree.#workspace(directory, store.environment.files);

        // place the dependencies and every build their locks link
        const builds = Object.values(dependencies);
        const named = new Set(builds.map((build) => build.manifest));
        const linked = new Set(Object.values(links).flatMap((each) => Object.values(each)));
        for (const digest of [...linked].filter((each) => !named.has(each))) {
            const { package: built } = await store.get(digest);
            builds.push(
                "id" in built
                    ? { kind: "destack", package: built, manifest: digest }
                    : { kind: "npm", package: built, manifest: digest },
            );
        }

        // resolve each requirement through its build's lock, else by name among the dependencies
        const resolve = (build: Digest, name: string) =>
            links[build]?.[name] ?? dependencies[name]?.manifest;
        const scope = { kind: "compile" } as const;

        // place them in the install's span, counting the builds
        return await store.scope.span(
            "destack.dependency.install",
            { "destack.dependency.builds": builds.length },
            () =>
                DependencyTree.materialize(store, builds, resolve, store.trees, scope, {
                    workspace,
                }),
        );
    }

    /** Write builds once each under a directory, each linked to the builds it requires. */
    static async materialize(
        store: LocalBuildStore,
        builds: readonly DependencyRelease[],
        resolve: (build: Digest, name: string) => Digest | undefined,
        directory: string,
        scope: TreeScope,
        options: {
            /** The workspace whose Bun store links its members' npm packages, else the entries. */
            readonly workspace?: string | undefined;
        } = {},
    ): Promise<DependencyTree> {
        // take the builds the tree places
        const { files, crypto } = store.environment;
        const { workspace } = options;
        const installation =
            workspace === undefined
                ? undefined
                : await DependencyTree.#installation(workspace, files);
        const placed = new Map(
            builds
                .filter((entry) => scope.kind === "compile" || scope.closure.has(entry.manifest))
                .map((entry) => [entry.manifest, entry] as const),
        );

        // read what each build links among the placed builds
        const read = new Map<
            Digest,
            {
                readonly reader: BuildReader;
                readonly required: readonly string[];
                readonly links: Readonly<Record<string, Digest>>;
            }
        >();
        for (const [manifest, entry] of placed) {
            const reader = await store.contents(manifest);
            const required = await DependencyTree.#required(reader, entry);
            const resolved = required.flatMap((each) => {
                const target = resolve(manifest, each);

                return target !== undefined && target !== manifest && placed.has(target)
                    ? [[each, target] as const]
                    : [];
            });
            read.set(manifest, { reader, required, links: Object.fromEntries(resolved) });
        }

        // key each root by its build and every edge of its tree
        const visits = new Map<Digest, TreeVisit>();
        for (const [manifest, entry] of placed) {
            const { reader, required, links } = present(read.get(manifest), manifest);
            const edges = DependencyTree.#edges(
                manifest,
                (each) => present(read.get(each), each).links,
            );
            const base =
                edges.length === 0 ? manifest : `${manifest}+${await Digest.json(edges, crypto)}`;
            const root = join(directory, DependencyTree.#rootName(base, scope));
            visits.set(manifest, {
                name: entry.package.name,
                entry,
                reader,
                root,
                required,
                links,
            });
        }

        // place each root once
        const done = new Set<Digest>();
        const place = async (manifest: Digest, visiting: ReadonlySet<Digest>): Promise<void> => {
            // place the linked roots first
            const visit = present(visits.get(manifest), manifest);
            if (done.has(manifest) || visiting.has(manifest)) {
                return;
            }
            for (const each of Object.values(visit.links)) {
                await place(each, new Set([...visiting, manifest]));
            }
            done.add(manifest);

            // place a root the trees lack
            if (!(await files.exists(visit.root))) {
                await DependencyTree.#place(store, visit, visits, directory, installation, scope);
            }
        };
        for (const manifest of visits.keys()) {
            await place(manifest, new Set());
        }

        // read the Destack builds' exports where each lies in its root
        const placedIn = (kind: DependencyRelease["kind"]): Record<string, string> =>
            Object.fromEntries(
                [...visits.values()]
                    .filter((visit) => visit.entry.kind === kind)
                    .map(
                        (visit) =>
                            [visit.name, join(visit.root, "node_modules", visit.name)] as const,
                    )
                    .toReversed(),
            );
        const tree = await DependencyTree.read(placedIn("destack"), files);

        return new DependencyTree(
            { ...placedIn("npm"), ...tree.directories },
            tree.roles,
            tree.#exports,
        );
    }

    /** List the packages a build requires at run time: an npm unit's requirements and peers. */
    static async #required(reader: BuildReader, entry: DependencyRelease): Promise<string[]> {
        // take an npm unit's requirements and the peers its host output declares
        if (entry.kind === "npm") {
            const peers = Object.keys(reader.manifest.outputs[HOST_RUNTIME]?.peers ?? {});

            return [...new Set([...Object.keys(reader.manifest.requires), ...peers])];
        }
        // take every runtime dependency a Destack build's package.json declares
        else {
            const authored = AuthoredPackage.parse(
                JSON.parse(new TextDecoder().decode(await reader.load("package.json"))),
            );

            return PackageJson.runtimeDependencies(authored);
        }
    }

    /** List every edge of a build's tree, as sorted from, name and target triples. */
    static #edges(
        root: Digest,
        links: (build: Digest) => Readonly<Record<string, Digest>>,
    ): string[] {
        // walk the links once each from the root
        const edges: string[] = [];
        const seen = new Set<Digest>([root]);
        const pending = [root];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            for (const [name, target] of Object.entries(links(next))) {
                edges.push(`${next} ${name} ${target}`);
                if (!seen.has(target)) {
                    seen.add(target);
                    pending.push(target);
                }
            }
        }

        return edges.toSorted();
    }

    /** Write one build into its root: its files, its links, and its package.json. */
    static async #place(
        store: LocalBuildStore,
        visit: TreeVisit,
        visits: ReadonlyMap<Digest, TreeVisit>,
        directory: string,
        installation: WorkspaceInstall | undefined,
        scope: TreeScope,
    ): Promise<void> {
        // stage the build beside its root
        const { files } = store.environment;
        const { name, entry, reader, root } = visit;
        const staged = await DependencyTree.#stage(directory, files);
        const target = join(staged, "node_modules", name);
        for (const [each, manifest] of Object.entries(visit.links)) {
            const linked = join(present(visits.get(manifest), manifest).root, "node_modules", each);
            await DependencyTree.#symlink(
                linked,
                join(staged, "node_modules", each),
                { staged, root },
                files,
            );
        }

        // place an npm unit's host output
        if (entry.kind === "npm") {
            const output = present(
                reader.manifest.outputs[HOST_RUNTIME],
                `the ${HOST_RUNTIME} output of ${name}`,
            );
            for (const [peer, declared] of Object.entries(output.peers)) {
                if (!declared.isOptional && visit.links[peer] === undefined) {
                    throw BuildError.of(
                        "DEPENDENCY_INVALID",
                        `peer '${peer}' of '${name}' is invalid: its dependent does not provide it`,
                    );
                }
            }
            const prefix = `${output.directory}/`;
            await store.place(
                (await DependencyTree.#placed(reader, scope))
                    .filter((file) => file.path.startsWith(prefix))
                    .map((file) => ({
                        file,
                        destination: join(target, file.path.slice(prefix.length)),
                    })),
            );
            // write a converted output's package.json
            const exports = Object.fromEntries(
                Object.entries(output.exports).map(([subpath, module]) => [
                    subpath,
                    `./${module.path.slice(prefix.length)}`,
                ]),
            );
            await files.mkdir(target, { recursive: true });
            if (!(await files.exists(join(target, "package.json")))) {
                await files.writeTextFile(
                    join(target, "package.json"),
                    JSON.stringify({ name, type: "module", exports }),
                );
            }
        }
        // place a Destack build's files
        else {
            const built = await DependencyTree.#placed(reader, scope);
            await store.place(
                built
                    .filter((file) => file.path !== "package.json")
                    .map((file) => ({ file, destination: join(target, file.path) })),
            );
            await files.writeFile(
                join(target, MANIFEST_PATH),
                PackageManifest.serialize(reader.manifest),
            );
            const authored = AuthoredPackage.parse(
                JSON.parse(new TextDecoder().decode(await reader.load("package.json"))),
            );
            if (installation !== undefined && installation.members.has(name)) {
                const installed = await DependencyTree.#installed(
                    installation,
                    name,
                    authored,
                    visit.links,
                );
                for (const [each, path] of installed) {
                    await DependencyTree.#symlink(
                        path,
                        join(target, "node_modules", each),
                        { staged, root },
                        files,
                    );
                }
            }
            const exports = DependencyTree.#exportsOf(
                Object.values(reader.manifest.outputs),
                authored.exports,
            );
            await files.writeTextFile(
                join(target, "package.json"),
                JSON.stringify({ ...authored, type: "module", exports }),
            );
        }

        // move it into place
        await DependencyTree.#move(staged, root, files);
    }

    /** Find the npm packages a member declares as the workspace installed them. */
    static async #installed(
        installation: WorkspaceInstall,
        owner: string,
        authored: schema.Infer<typeof AuthoredPackage>,
        links: Readonly<Record<string, Digest>>,
    ): Promise<Map<string, string>> {
        // find each runtime dependency the tree does not place
        const linked = new Map<string, string>();
        for (const name of PackageJson.runtimeDependencies(authored)) {
            const version =
                authored.dependencies?.[name] ??
                authored.optionalDependencies?.[name] ??
                authored.peerDependencies?.[name];
            const isPlaced = links[name] !== undefined;
            const placed =
                isPlaced || version === undefined || version.startsWith("workspace:")
                    ? undefined
                    : await installation.find(
                          owner,
                          name,
                          { name, version },
                          !PackageJson.isOptional(authored, name),
                      );
            if (placed !== undefined) {
                linked.set(name, placed);
            }
        }

        return linked;
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

    /** List a build's files a tree places: a whole build for a compile, else what a run runs. */
    static async #placed(reader: BuildReader, scope: TreeScope): Promise<readonly PackageFile[]> {
        // place every file for a compile
        if (scope.kind === "compile") {
            return await reader.distributed();
        }
        // place the output a running tree runs, refusing a build without one
        else {
            present(
                Object.values(reader.manifest.outputs).find(
                    (output) => output.runtime === scope.runtime,
                ),
                `the ${scope.runtime} output of ${reader.manifest.package.name}`,
            );

            return await reader.runFiles([scope.runtime]);
        }
    }

    /** Read the build a tree root holds by its name, absent for a name of no root. */
    static manifestOf(name: string): Digest | undefined {
        const [manifest] = name.split(/[+.]/u);
        const parsed = Digest.safeParse(manifest);

        return parsed.success ? parsed.data : undefined;
    }

    /** Name a tree root: a build and its edges, then what the tree places. */
    static #rootName(base: string, scope: TreeScope): string {
        return scope.kind === "compile" ? base : `${base}.${scope.runtime}`;
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

    /** The directory of each Destack dependency's build, by package name. */
    get builds(): Readonly<Record<string, string>> {
        return Object.fromEntries(
            Object.entries(this.directories).filter(([name]) => this.roles.has(name)),
        );
    }

    /** Read the tree of materialized dependency directories, by dependency name. */
    static async read(
        directories: Readonly<Record<string, string>>,
        files: FileSystem,
    ): Promise<DependencyTree> {
        // read each package's exports, and its role where it declares one
        const roles = new Map<string, PackageRole>();
        const exports = new Map<string, schema.Infer<typeof MaterializedExports>>();
        for (const [name, directory] of Object.entries(directories)) {
            // read a Destack package's role and runtime exports, an npm package having neither
            const definition = join(directory, DEFINITION_FILE);
            if (await files.exists(definition)) {
                const packageJson = MaterializedPackage.parse(
                    JSON.parse(await files.readTextFile(join(directory, "package.json"))),
                );
                exports.set(name, packageJson.exports);
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
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `subpath '${subpath}' is invalid: '${name}' does not export it`,
            );
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
            if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
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
                "DEPENDENCY_INVALID",
                `npm package '${name}@${version}' is invalid: it is not installed`,
            );
        } else if (others.length > 0) {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `npm package is invalid: it is installed with several peer sets '${entries.join(", ")}'`,
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
                "INVARIANT_BROKEN",
                `the dependency tree has no directory for '${name}'`,
            );
        }

        return directory;
    }
}
