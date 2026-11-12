import { dirname, join, resolve } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import type { BunLockfile } from "@destack/check/bun";
import { DomainError } from "@destack/error";
import {
    type DependencyRelease,
    type PackageVersion,
    PackageJson,
    PackageName,
    type graph,
} from "@destack/package";
import { type BuildReader, Lock, PackageManifest } from "@destack/package/manifest";
import { PackageLocator } from "@destack/package/locator";
import type { Runtime } from "@destack/package/runtime";
import { Digest, found, nth, present, Version } from "@destack/schema";
import { Checkout } from "../cache/index.ts";
import { NpmInstallation, type NpmUnit } from "./npm.ts";
import { outputsOf, type PackageBuildOptions } from "../build/build.ts";
import type { Compiler } from "../build/compiler.ts";
import type { CompilerPool } from "../build/pool.ts";
import type { InspectOptions } from "../inspection/inspection.ts";
import type { PackageInspection } from "@destack/package/code";
import { BuildError } from "../error/index.ts";
import { readLockedReleases, readLockfile, readPackageJson } from "../source/index.ts";
import { BuildStore } from "../store/index.ts";
import type { LocalBuildStore } from "../store/local.ts";
import type { DependencyBuild } from "./tree.ts";

/** A Destack package of a workspace, built as one unit on its dependencies' builds. */
interface WorkspaceUnit {
    /** The package's name. */
    readonly name: string;
    /** The package's directory. */
    readonly directory: string;
    /** The package's package.json and destack.json, as read at open. */
    readonly packageJson: PackageJson;
    /** The names of the workspace units it requires. */
    readonly requires: readonly string[];
    /** The Destack packages it requires from outside the workspace, built as releases. */
    readonly releases: readonly PackageVersion[];
    /** The npm packages it requires at runtime, by import name, at their real directories. */
    readonly npm: ReadonlyMap<string, string>;
}

/** A build of a workspace's root, sharing its deadline, versions and stamping with its units. */
export type WorkspaceBuildOptions = Omit<
    PackageBuildOptions,
    "directory" | "dependencies" | "links"
> & {
    /** Whether every unit is stamped as a dev build, as Go's pseudo-versions are. */
    readonly isPrerelease?: boolean;
};

/** What a workspace's units share when stored: cancellation, deadline, versions, stamping. */
export type WorkspaceUnitOptions = Pick<
    WorkspaceBuildOptions,
    "signal" | "timeout" | "versions" | "isPrerelease"
>;

/** The compilers a workspace builds with, and the store they build into. */
export type WorkspaceCompilers = Pick<CompilerPool, "store" | "fingerprint" | "use">;

/** Find a release outside the workspace by its lock's digest, storing its lock and builds. */
export type ReleaseResolver = (dependency: PackageVersion) => Promise<Digest>;

/** A release the workspace requires: its lock, its root build and where it comes from. */
export interface WorkspaceRelease {
    /** The release's lock. */
    readonly lock: Lock;
    /** The release's root build, which its dependents compile against. */
    readonly root: DependencyRelease;
    /** Where the release comes from: the host as a builtin, or the resolver. */
    readonly source: "host" | "resolver";
}

/** Which workspace members a build compiles from source: all, or the publishing account's. */
export type WorkspaceSelection = "all" | "account";

/** How a workspace opens: its release resolver, the members it compiles and the host's builtins. */
export interface WorkspaceOptions {
    /** Find the releases the units require from outside the workspace, absent for none. */
    readonly resolver?: ReleaseResolver;
    /** Which members a build compiles from source, all by default. */
    readonly selection?: WorkspaceSelection;
    /** The locks of the builtins the host provides, none by default. */
    readonly host?: readonly Lock[];
}

/** The Destack packages of a checkout as read once, each built on its dependencies' builds. */
export class Workspace {
    /** The units of the root package's closure, each after the units it requires, the root last. */
    readonly units: readonly WorkspaceUnit[];
    /** The package the workspace builds, on the builds of the units before it. */
    readonly root: WorkspaceUnit;
    /** The units, by package name. */
    readonly #named: ReadonlyMap<string, WorkspaceUnit>;
    /** The compilers the units compile with, and the store they build into. */
    readonly #compilers: WorkspaceCompilers;
    /** The fingerprint of the toolchain building the units. */
    readonly #toolchain: Digest;
    /** The files of the git checkout enclosing the workspace, which the units' keys read. */
    readonly #checkout: Checkout;
    /** The npm packages the lockfile locks, each stored once as a unit. */
    readonly #npm: NpmInstallation;
    /** Find the releases the units require from outside the workspace, absent for none. */
    readonly #resolver: ReleaseResolver | undefined;
    /** The locks of the builtins the host provides. */
    readonly #host: readonly Lock[];
    /** The releases resolved once a build first needs them, each with its lock and root. */
    #releases: Promise<ReadonlyMap<string, WorkspaceRelease>> | undefined;

    /** Store a closure's units in build order, the root last, with what their keys read. */
    private constructor(
        units: readonly WorkspaceUnit[],
        compilers: WorkspaceCompilers,
        toolchain: Digest,
        checkout: Checkout,
        npm: NpmInstallation,
        resolver: ReleaseResolver | undefined,
        host: readonly Lock[],
    ) {
        // store the units and the compilers
        this.units = units;
        this.root = nth(units, units.length - 1);
        this.#named = new Map(units.map((unit) => [unit.name, unit]));
        this.#compilers = compilers;

        // store what the units' keys read
        this.#toolchain = toolchain;
        this.#checkout = checkout;
        this.#npm = npm;
        this.#resolver = resolver;
        this.#host = host;
    }

    /** Read the closure of Destack workspace members a package requires, in build order. */
    static async open(
        directory: string,
        compilers: WorkspaceCompilers,
        options: WorkspaceOptions = {},
    ): Promise<Workspace> {
        // read the workspace's members by their real directories, refusing a missing one
        const { resolver, selection = "all", host = [] } = options;
        const { environment } = compilers.store;
        const { files } = environment;
        const lockfile = await readLockfile(directory, files);
        const workspace = await Workspace.#locate(lockfile, files);
        const members = new Set(
            await Promise.all(
                Object.entries(lockfile.workspaces).map(async ([path, member]) => {
                    const location = join(workspace, path);

                    return await files.realPath(location).catch((cause: unknown) => {
                        if (!FileSystemError.is(cause, "FILE_NOT_FOUND")) {
                            throw cause;
                        }
                        throw BuildError.of(
                            "MEMBER_NOT_FOUND",
                            `workspace member '${member.name ?? path}' at '${location}' not found`,
                            { cause },
                        );
                    });
                }),
            ),
        );

        // walk the Destack members the root requires through their installed links
        const units = new Map<string, WorkspaceUnit>();
        const locator = new PackageLocator(files);
        const root = await files.realPath(directory);
        const account = PackageName.split((await readPackageJson(root, files)).package.name).handle;
        const compiled = (name: string) =>
            selection === "all" || PackageName.split(name).handle === account;
        const walk = { locator, files, members, compiled, units, visiting: new Set<string>() };
        await Workspace.#visit(root, walk);

        // read the toolchain, the checkout and the locked npm releases the units' keys read
        const toolchain = await compilers.fingerprint.digest();
        const checkout = await Checkout.list(workspace, environment);
        const locked = readLockedReleases(lockfile);
        const conversion = await compilers.fingerprint.conversion();
        const npm = new NpmInstallation(locked, compilers.store, conversion, environment);

        return new Workspace(
            [...units.values()],
            compilers,
            toolchain,
            checkout,
            npm,
            resolver,
            host,
        );
    }

    /** Resolve each release the units require once a build first needs them. */
    #resolvedReleases(): Promise<ReadonlyMap<string, WorkspaceRelease>> {
        this.#releases ??= this.#resolveReleases();

        return this.#releases;
    }

    /** Resolve each release the units require, and each builtin the host provides, to its lock. */
    async #resolveReleases(): Promise<ReadonlyMap<string, WorkspaceRelease>> {
        // take each package the host provides as a builtin from its lock
        const releases = new Map<string, WorkspaceRelease>();
        for (const lock of this.#host) {
            const { name } = (await this.#store.get(lock.root)).package;
            const root = present((await this.#store.entriesOf(lock))[name], `the root of ${name}`);
            releases.set(name, { lock, root, source: "host" });
        }

        // refuse the other required releases to a workspace without a resolver
        const resolver = this.#resolver;
        const required = new Map(
            this.units
                .flatMap((unit) => unit.releases)
                .filter((each) => !releases.has(each.name))
                .map((each) => [each.name, each] as const),
        );
        if (required.size > 0 && resolver === undefined) {
            const names = [...required.keys()].toSorted().join(", ");
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `workspace is invalid: it requires Destack packages '${names}' outside it and has no release resolver for them`,
            );
        }

        // resolve each other release to its lock and its root build
        for (const dependency of required.values()) {
            const lock = await this.#store.lock(
                await present(resolver, "the release resolver")(dependency),
            );
            const entries = await this.#store.entriesOf(lock);
            const root = present(entries[dependency.name], `the root of ${dependency.name}`);
            releases.set(dependency.name, { lock, root, source: "resolver" });
        }

        return releases;
    }

    /** Visit a member and the Destack members it requires, each before the members requiring it. */
    static async #visit(
        from: string,
        walk: {
            readonly locator: PackageLocator;
            readonly files: FileSystem;
            readonly members: ReadonlySet<string>;
            readonly compiled: (name: string) => boolean;
            readonly units: Map<string, WorkspaceUnit>;
            readonly visiting: Set<string>;
        },
    ): Promise<WorkspaceUnit> {
        // take a visited unit, refusing a cycle through one being visited
        const known = walk.units.get(from);
        if (known !== undefined) {
            return known;
        }
        if (walk.visiting.has(from)) {
            throw BuildError.of(
                "DEPENDENCY_INVALID",
                `workspace dependency '${from}' is invalid: it is part of a dependency cycle`,
            );
        }
        walk.visiting.add(from);
        const packageJson = await readPackageJson(from, walk.files);
        const name = packageJson.package.name;

        // split each runtime or listed dependency once into units and npm packages, leaving the rest to the compiler
        const listed = packageJson.definition.extensions ?? [];
        const requires: string[] = [];
        const releases: PackageVersion[] = [];
        const npm = new Map<string, string>();
        for (const dependency of new Set([
            ...PackageJson.runtimeDependencies(packageJson),
            ...listed,
        ])) {
            const installed = await walk.locator.directory(dependency, from);
            const resolved =
                installed === undefined ? undefined : await walk.files.realPath(installed);
            const isOptional =
                !listed.includes(dependency) && PackageJson.isOptional(packageJson, dependency);
            const isDestack =
                resolved !== undefined && (await walk.files.exists(join(resolved, "destack.json")));
            const isMember =
                resolved !== undefined &&
                walk.members.has(resolved) &&
                (!isDestack || walk.compiled(dependency));
            // refuse a missing dependency the package cannot build without
            if (resolved === undefined && !isOptional) {
                throw BuildError.of(
                    "DEPENDENCY_INVALID",
                    `dependency '${dependency}' of '${name}' is invalid: it is missing`,
                );
            }
            // build a Destack member as a unit
            else if (resolved !== undefined && isMember && isDestack) {
                requires.push((await Workspace.#visit(resolved, walk)).name);
            }
            // take a Destack package from outside the workspace as a release
            else if (resolved !== undefined && !isMember && isDestack) {
                releases.push((await readPackageJson(resolved, walk.files)).package);
            }
            // take an npm package from outside the workspace, stored as npm units
            else if (resolved !== undefined && !isMember && !isDestack) {
                npm.set(dependency, resolved);
            }
        }
        const visited = { name, directory: from, packageJson, requires, releases, npm };
        walk.units.set(from, visited);

        return visited;
    }

    /** Find the workspace a lockfile belongs to, past a link to it. */
    static async #locate(lockfile: BunLockfile, files: FileSystem): Promise<string> {
        return dirname(await files.realPath(join(lockfile.directory, "bun.lock")));
    }

    /** The store the workspace's units build into. */
    get #store(): LocalBuildStore {
        return this.#compilers.store;
    }

    /** Build the root on its dependencies' builds with the given outputs, read from the store. */
    async build(request: WorkspaceBuildOptions): Promise<BuildReader> {
        // build the dependencies
        const { signal, timeout, versions, isPrerelease = false, ...given } = request;
        const shared = {
            ...(signal === undefined ? {} : { signal }),
            ...(timeout === undefined ? {} : { timeout }),
            ...(versions === undefined ? {} : { versions }),
        };
        const stored = await this.#storeEach({ ...shared, isPrerelease }, this.#dependencies);

        // take the root's stored build with the given outputs
        const dependencies = await this.#inputs(this.root, stored);
        const options = await this.#options(this.root, { ...shared, ...given }, dependencies);
        const manifest = await this.#cachedPut(this.root, options, isPrerelease);
        await this.#store.rootDirectory(this.root.directory, new Set([manifest]));

        return await this.#store.contents(manifest);
    }

    /** Lock the root's build with every build it runs with: units, releases and npm units. */
    async lock(root: Digest, shared: WorkspaceUnitOptions = {}): Promise<Digest> {
        const units = await this.#storeEach(shared, this.#dependencies);

        return await this.#lock(this.root, root, units);
    }

    /** Build and lock every unit, by name. */
    async lockUnits(
        shared: WorkspaceUnitOptions,
    ): Promise<ReadonlyMap<string, { readonly build: DependencyBuild; readonly lock: Digest }>> {
        // build every unit once, then lock each
        const units = await this.#storeEach(shared, this.units);
        const locks = new Map<string, { readonly build: DependencyBuild; readonly lock: Digest }>();
        for (const unit of this.units) {
            const build = found(units, unit.name);
            locks.set(unit.name, { build, lock: await this.#lock(unit, build.manifest, units) });
        }

        return locks;
    }

    /** Lock a unit's build with every build it runs with. */
    async #lock(
        unit: WorkspaceUnit,
        root: Digest,
        units: ReadonlyMap<string, DependencyBuild>,
    ): Promise<Digest> {
        // index the Destack builds the unit compiles against, apart from builtins
        const destack = new Map<string, Digest>();
        const builtins = new Set<string>();
        for (const [name, dependency] of Object.entries(await this.#inputs(unit, units))) {
            if ((await this.#store.get(dependency.manifest)).role === "builtin") {
                builtins.add(name);
            } else {
                destack.set(name, dependency.manifest);
            }
        }

        // index the workspace units by build
        const members = new Map<Digest, WorkspaceUnit>([[root, unit]]);
        for (const member of this.#closureOf(unit)) {
            members.set(found(units, member.name).manifest, member);
        }
        const released = await this.#releaseLinks();

        // link each build from the root on to the build each of its requirements resolves to
        const index = {
            destack,
            builtins,
            members,
            released,
            installed: new Map<Digest, NpmUnit>(),
        };
        const builds = new Map<Digest, Readonly<Record<string, Digest>>>();
        const pending: Digest[] = [root];
        for (const digest of pending) {
            if (!builds.has(digest)) {
                const links = await this.#links(digest, index);
                builds.set(digest, links);
                pending.push(...Object.values(links));
            }
        }

        // resolve each peer a unit's install left to its dependents from a build linking the unit
        const linking = new Map<Digest, Digest[]>();
        for (const [from, links] of builds) {
            for (const to of Object.values(links)) {
                linking.set(to, [...(linking.get(to) ?? []), from]);
            }
        }
        for (const digest of index.installed.keys()) {
            builds.set(digest, await this.#peers(digest, builds, linking.get(digest) ?? []));
        }

        return await this.#store.putLock(Lock.of(root, Object.fromEntries(builds)));
    }

    /** Resolve a locked build's requirements: a release's from its lock, else its installs. */
    async #links(
        digest: Digest,
        index: {
            readonly destack: ReadonlyMap<string, Digest>;
            readonly builtins: ReadonlySet<string>;
            readonly members: ReadonlyMap<Digest, WorkspaceUnit>;
            readonly released: Readonly<Record<Digest, Readonly<Record<string, Digest>>>>;
            readonly installed: Map<Digest, NpmUnit>;
        },
    ): Promise<Readonly<Record<string, Digest>>> {
        // find what the build is
        const release = index.released[digest];
        const unit = index.installed.get(digest);
        const links: Record<string, Digest> = {};

        // link a release's build
        if (release !== undefined) {
            return release;
        }
        // link an npm unit to what its build requires
        else if (unit !== undefined) {
            const manifest = await this.#store.get(digest);
            const peers = Object.values(manifest.outputs).flatMap((output) =>
                Object.keys(output.peers),
            );
            for (const name of new Set([...Object.keys(manifest.requires), ...peers])) {
                const dependency = unit.dependencies.get(name);
                if (dependency !== undefined && !index.builtins.has(name)) {
                    links[name] = await this.#installed(dependency, index.installed);
                }
            }
        }
        // link a workspace unit to Destack builds and the npm packages it installs
        else {
            const member = present(index.members.get(digest), `the workspace unit of ${digest}`);
            const required = Object.keys((await this.#store.get(digest)).requires);
            for (const name of required.filter((each) => !index.builtins.has(each))) {
                const directory = member.npm.get(name);
                const destack = index.destack.get(name);
                if (destack !== undefined) {
                    links[name] = destack;
                } else if (directory !== undefined) {
                    const dependency = await this.#npm.open(directory);
                    links[name] = await this.#installed(dependency, index.installed);
                }
            }
        }

        return links;
    }

    /** Link a unit's missing peers to what a dependent links, or to the dependent itself. */
    async #peers(
        digest: Digest,
        builds: ReadonlyMap<Digest, Readonly<Record<string, Digest>>>,
        dependents: readonly Digest[],
    ): Promise<Readonly<Record<string, Digest>>> {
        // find the peers of the unit's build its links leave out
        const links = { ...builds.get(digest) };
        const manifest = await this.#store.get(digest);
        const peers = Object.values(manifest.outputs).flatMap((output) =>
            Object.keys(output.peers),
        );
        for (const peer of peers.filter((each) => links[each] === undefined)) {
            // take the first dependent providing the peer
            for (const dependent of dependents) {
                const isPeer = (await this.#store.get(dependent)).package.name === peer;
                const provided = isPeer ? dependent : builds.get(dependent)?.[peer];
                if (provided !== undefined && links[peer] === undefined) {
                    links[peer] = provided;
                }
            }
        }

        return links;
    }

    /** Store an npm unit's build once, recording which unit each stored build is. */
    async #installed(unit: NpmUnit, installed: Map<Digest, NpmUnit>): Promise<Digest> {
        const { manifest } = await this.#npm.store(unit);
        installed.set(manifest, unit);

        return manifest;
    }

    /** Inspect the root against its dependencies' builds, reusing stored units. */
    async inspect(request: Omit<InspectOptions, "directory">): Promise<PackageInspection> {
        return await this.#against(
            request.runtime,
            async (compiler, dependencies) =>
                await compiler.inspect({
                    ...request,
                    directory: this.root.directory,
                    dependencies,
                    links: await this.#releaseLinks(),
                }),
        );
    }

    /** Index the root's code against its dependencies' builds, reusing stored units. */
    async index(
        request: Omit<InspectOptions, "directory">,
    ): Promise<{ code: graph.Document[]; modules: graph.Module[] }> {
        return await this.#against(
            request.runtime,
            async (compiler, dependencies) =>
                await compiler.index({
                    ...request,
                    directory: this.root.directory,
                    dependencies,
                    links: await this.#releaseLinks(),
                }),
        );
    }

    /** Store every unit before the root for its runtimes, answering the root's compile inputs. */
    async storeDependencies(
        shared: WorkspaceUnitOptions,
    ): Promise<Record<string, DependencyRelease>> {
        const stored = await this.#storeEach(shared, this.#dependencies);

        return await this.#inputs(this.root, stored);
    }

    /** Run work with the root's compiler against the builds of its dependencies. */
    async #against<Result>(
        runtime: Runtime,
        work: (
            compiler: Compiler,
            dependencies: Record<string, DependencyRelease>,
        ) => Promise<Result>,
    ): Promise<Result> {
        const dependencies = await this.storeDependencies({});

        return await this.#compilers.use(this.root.directory, (compiler) =>
            work(compiler, dependencies),
        );
    }

    /** The units before the root. */
    get #dependencies(): readonly WorkspaceUnit[] {
        return this.units.filter((unit) => unit !== this.root);
    }

    /** Store some units, each with every output it ships, after the units it requires. */
    async #storeEach(
        shared: WorkspaceUnitOptions,
        units: readonly WorkspaceUnit[],
    ): Promise<ReadonlyMap<string, DependencyBuild>> {
        // start every unit at once after the units it requires, as the pool leases compilers in turn
        const started = new Map<string, Promise<DependencyBuild>>();
        for (const unit of units) {
            started.set(unit.name, this.#storeUnit(unit, started, shared));
        }
        // let every started unit settle, as Cargo drains running jobs, then refuse the first failure
        const settled = await Promise.allSettled(
            [...started].map(async ([name, unit]) => [name, await unit] as const),
        );
        const failed = settled.find((result) => result.status === "rejected");
        if (failed !== undefined) {
            throw failed.reason;
        }

        return new Map(
            settled.flatMap((result) => (result.status === "fulfilled" ? [result.value] : [])),
        );
    }

    /** Take a unit's stored build of every output it ships, building it first when missing. */
    async #storeUnit(
        unit: WorkspaceUnit,
        started: ReadonlyMap<string, Promise<DependencyBuild>>,
        shared: WorkspaceUnitOptions,
    ): Promise<DependencyBuild> {
        // wait for the units it requires
        const required = new Map<string, DependencyBuild>();
        for (const name of this.#closureOf(unit).map((each) => each.name)) {
            required.set(name, await found(started, name));
        }
        const dependencies = await this.#inputs(unit, required);

        // take its stored build of every output it ships
        const outputs = outputsOf(unit.packageJson);
        const { isPrerelease = false, ...given } = shared;
        const options = await this.#options(unit, { ...given, outputs }, dependencies);
        const manifest = await this.#cachedPut(unit, options, isPrerelease);
        const built = PackageManifest.package(await this.#store.get(manifest));

        // root the build as the unit's latest
        await this.#store.rootDirectory(unit.directory, new Set([manifest]));

        return { kind: "destack", package: built, manifest };
    }

    /** Take a unit's stored build, building it once and refusing sources edited meanwhile. */
    async #cachedPut(
        unit: WorkspaceUnit,
        options: PackageBuildOptions,
        isPrerelease: boolean,
    ): Promise<Digest> {
        // key the unit by its sources as they read now
        const sources = await this.#checkout.identify(unit.directory);
        const key = await this.#key(unit, options, sources, isPrerelease);

        // build it once when missing
        return await this.#store.cachedBuild(key, async () => {
            // stamp a dev build at its time and key
            const at = this.#store.environment.clock.now();
            const version = Version.dev(unit.packageJson.package.version, at, key);
            const versions = { ...Workspace.#versionsOf(options), [unit.name]: version };
            const stamped = isPrerelease ? { ...options, versions } : options;

            // evaluate its declarations and compile it on them, in one request
            const build = await this.#use(unit, (compiler) => compiler.build(stamped));
            const built = await BuildStore.digest(build.manifest, this.#store.environment.crypto);
            const listed = await Checkout.list(
                unit.directory,
                this.#store.environment,
                "directory",
            );
            if ((await listed.identify(unit.directory)) !== sources) {
                throw BuildError.of(
                    "SOURCE_CONFLICT",
                    `cannot build '${unit.name}': it changed while it built`,
                );
            }

            return built;
        });
    }

    /** List the units a unit requires, and theirs, each once. */
    #closureOf(unit: WorkspaceUnit): WorkspaceUnit[] {
        // walk the requirements once each
        const closure = new Map<string, WorkspaceUnit>();
        const pending = [...unit.requires];
        for (let name = pending.pop(); name !== undefined; name = pending.pop()) {
            if (!closure.has(name)) {
                const required = found(this.#named, name);
                closure.set(name, required);
                pending.push(...required.requires);
            }
        }

        return [...closure.values()];
    }

    /** Read what a unit compiles against: the root builds of its builtins, units and releases. */
    async #inputs(
        unit: WorkspaceUnit,
        stored: ReadonlyMap<string, DependencyBuild>,
    ): Promise<Record<string, DependencyRelease>> {
        // take the host's builtins, then each required unit's build, then each required release's root
        const closure = this.#closureOf(unit);
        const releases = await this.#resolvedReleases();
        const inputs: Record<string, DependencyRelease> = {};
        for (const [name, release] of releases) {
            if (release.source === "host") {
                inputs[name] = release.root;
            }
        }
        for (const each of closure) {
            inputs[each.name] = found(stored, each.name);
        }
        for (const owner of [unit, ...closure]) {
            for (const release of owner.releases) {
                inputs[release.name] = found(releases, release.name).root;
            }
        }

        return inputs;
    }

    /** List the npm releases a unit's npm dependencies lock, which its key reads. */
    async #npmReleases(unit: WorkspaceUnit): Promise<string[]> {
        // open each npm dependency and list its closure's releases by name, version and integrity
        const releases = new Set<string>();
        for (const directory of unit.npm.values()) {
            for (const each of (await this.#npm.open(directory)).closure()) {
                const { package: released, integrity } = each.release;
                releases.add(`${released.name}@${released.version} ${integrity}`);
            }
        }

        return [...releases].toSorted();
    }

    /** Read the versions a unit's compile releases its packages at, dependencies' then given. */
    static #versionsOf(options: PackageBuildOptions): Record<string, Version> {
        const built = Object.entries(options.dependencies).flatMap(([name, dependency]) =>
            dependency.kind === "destack"
                ? [[name, Version.parse(dependency.package.version)] as const]
                : [],
        );

        return { ...Object.fromEntries(built), ...options.versions };
    }

    /** Take a unit's build options on its dependencies, with the links of their locks. */
    async #options(
        unit: WorkspaceUnit,
        request: WorkspaceBuildOptions,
        dependencies: Record<string, DependencyRelease>,
    ): Promise<PackageBuildOptions> {
        return {
            ...request,
            directory: unit.directory,
            dependencies,
            links: await this.#releaseLinks(),
        };
    }

    /** Read the links of every release's lock, the host's builtins among them, by build. */
    async #releaseLinks(): Promise<Readonly<Record<Digest, Readonly<Record<string, Digest>>>>> {
        const releases = [...(await this.#resolvedReleases()).values()];

        return Object.fromEntries(releases.flatMap(({ lock }) => Object.entries(lock.builds)));
    }

    /** Key a unit's build by its sources, toolchain and its dependencies' interfaces. */
    async #key(
        unit: WorkspaceUnit,
        options: Omit<PackageBuildOptions, "signal" | "timeout">,
        checkout: Digest,
        isPrerelease: boolean,
    ): Promise<Digest> {
        // read each dependency's interface
        const interfaces = await Promise.all(
            Object.entries(options.dependencies).map(
                async ([name, dependency]) =>
                    [name, (await this.#store.get(dependency.manifest)).interface] as const,
            ),
        );

        return await Digest.json(
            {
                kind: "unit",
                ...(await this.#sources(unit, options, checkout)),
                isPrerelease,
                interfaces: Object.fromEntries(interfaces),
            },
            this.#store.environment.crypto,
        );
    }

    /** Read what a unit's build reads of its own: checkout, toolchain, outputs and release. */
    async #sources(
        unit: WorkspaceUnit,
        options: Omit<PackageBuildOptions, "signal" | "timeout">,
        checkout: Digest,
    ): Promise<Record<string, unknown>> {
        return {
            toolchain: this.#toolchain,
            conversion: this.#npm.conversion,
            checkout,
            outputs: options.outputs,
            configuration:
                options.configuration === undefined
                    ? null
                    : await this.#digestFile(resolve(unit.directory, options.configuration)),
            version: options.versions?.[unit.name] ?? null,
            npm: await this.#npmReleases(unit),
        };
    }

    /** Run work with a unit's compiler, reporting an undeclared failure as the unit's fault. */
    async #use<Result>(
        unit: WorkspaceUnit,
        work: (compiler: Compiler) => Promise<Result>,
    ): Promise<Result> {
        return await this.#compilers.use(unit.directory, work).catch((cause: unknown) => {
            // keep a declared failure as it is
            if (DomainError.is(cause)) {
                throw cause;
            }
            // report any other as an internal fault of the unit's build
            else {
                throw BuildError.of("INVARIANT_BROKEN", `building '${unit.name}' failed`, {
                    cause,
                });
            }
        });
    }

    /** Digest a file's bytes, null for a missing file. */
    async #digestFile(path: string): Promise<string | null> {
        const { files, crypto } = this.#compilers.store.environment;
        try {
            return await Digest.of(await files.readFile(path), crypto);
        } catch (error) {
            if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                return null;
            }
            throw error;
        }
    }
}
