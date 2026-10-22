import { existsSync } from "node:fs";
import { mkdtemp, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import type { BunLockfile } from "@destack/check/bun";
import { type DependencyResolution, PackageJson, type graph } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { PackageLocator } from "@destack/package/locator";
import type { History } from "@destack/resource";
import { Digest, found, nth, Text } from "@destack/schema";
import { Checkout, Identities, identifyToolchain } from "../cache/index.ts";
import { type NpmConverter, type NpmDependency, NpmInstallation } from "./npm.ts";
import { outputsOf, type PackageBuildOptions, PackageBuild } from "../build/build.ts";
import type { Compiler } from "../build/compiler.ts";
import type { CompilerPool } from "../build/pool.ts";
import type { InspectOptions } from "../inspection/inspection.ts";
import type { PackageInspection } from "@destack/package/code";
import { BuildError, isMissing } from "../error/index.ts";
import { readLockedReleases, readLockfile, readPackageJson } from "../source/index.ts";
import { PackageStore } from "../store/index.ts";
import type { LocalPackageStore } from "../store/local.ts";
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
    /** The npm packages it requires at runtime, by import name, at their real directories. */
    readonly npm: ReadonlyMap<string, string>;
}

/** A unit's stored build, with the npm releases its own npm dependencies reach. */
interface StoredUnit {
    /** The unit's stored build. */
    readonly build: DependencyBuild;
    /** The npm releases its own npm dependencies reach, by `name@version`. */
    readonly releases: Readonly<Record<string, NpmDependency>>;
}

/** A build of a workspace's root, sharing its deadline, versions and histories with its units. */
export interface WorkspaceBuildOptions extends Omit<
    PackageBuildOptions,
    "directory" | "dependencies" | "history"
> {
    /** What each source package has published, by package name, to plan its upgrade from. */
    history?: Readonly<Record<string, History>>;
}

/** What a workspace's units share when stored: cancellation, deadline, versions and histories. */
export type WorkspaceUnitOptions = Pick<
    WorkspaceBuildOptions,
    "signal" | "timeout" | "versions" | "history"
>;

/** The compilers a workspace builds with, and the store they build into. */
export type WorkspaceCompilers = Pick<CompilerPool, "store" | "use">;

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
    /** The identities of the packages and files the units' keys read. */
    readonly #identities: Identities;
    /** The identity of the toolchain building the units. */
    readonly #toolchain: Digest;
    /** The files of the git checkout enclosing the workspace, which the units' keys read. */
    readonly #checkout: Checkout;
    /** The npm packages the lockfile pins, each stored once as a unit. */
    readonly #npm: NpmInstallation;

    /** Hold a closure's units in build order, the root last, with what their keys read. */
    private constructor(
        units: readonly WorkspaceUnit[],
        compilers: WorkspaceCompilers,
        identities: Identities,
        toolchain: Digest,
        checkout: Checkout,
        npm: NpmInstallation,
    ) {
        // keep the units and the compilers
        this.units = units;
        this.root = nth(units, units.length - 1);
        this.#named = new Map(units.map((unit) => [unit.name, unit]));
        this.#compilers = compilers;

        // keep what the units' keys read
        this.#identities = identities;
        this.#toolchain = toolchain;
        this.#checkout = checkout;
        this.#npm = npm;
    }

    /** Read the closure of Destack workspace members a package requires, in build order. */
    static async open(directory: string, compilers: WorkspaceCompilers): Promise<Workspace> {
        // read the workspace's members by their real directories, refusing a missing one
        const lockfile = await readLockfile(directory);
        const workspace = await Workspace.#locate(lockfile);
        const members = new Set(
            await Promise.all(
                Object.entries(lockfile.workspaces).map(async ([path, member]) => {
                    const location = join(workspace, path);

                    return await realpath(location).catch((cause: unknown) => {
                        if (!isMissing(cause)) {
                            throw cause;
                        }
                        throw new BuildError(
                            "BUILD_FAILED",
                            `the workspace member ${member.name ?? path} is missing at ${location}`,
                            { cause },
                        );
                    });
                }),
            ),
        );

        // walk the Destack members the root requires through their installed links
        const units = new Map<string, WorkspaceUnit>();
        const walk = { locator: new PackageLocator(), members, units, visiting: new Set<string>() };
        await Workspace.#visit(await realpath(directory), walk);

        // read the toolchain, the checkout and the npm pins the units' keys read
        const identities = Identities.of({});
        const toolchain = await identifyToolchain(identities);
        const checkout = await Checkout.list(workspace);
        const npm = new NpmInstallation(readLockedReleases(lockfile), compilers.store, toolchain);

        return new Workspace([...units.values()], compilers, identities, toolchain, checkout, npm);
    }

    /** Visit a member and the Destack members it requires, each before the members requiring it. */
    static async #visit(
        from: string,
        walk: {
            readonly locator: PackageLocator;
            readonly members: ReadonlySet<string>;
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
            throw new BuildError("BUILD_FAILED", `workspace dependency cycle through ${from}`);
        }
        walk.visiting.add(from);
        const packageJson = await readPackageJson(from);
        const name = packageJson.package.name;

        // split each runtime or listed dependency once into units and npm packages, leaving the rest to the compiler
        const listed = packageJson.definition.extensions ?? [];
        const requires: string[] = [];
        const npm = new Map<string, string>();
        for (const dependency of new Set([
            ...PackageJson.runtimeDependencies(packageJson),
            ...listed,
        ])) {
            const installed = walk.locator.directory(dependency, from);
            const resolved = installed === undefined ? undefined : await realpath(installed);
            const isOptional =
                !listed.includes(dependency) && PackageJson.isOptional(packageJson, dependency);
            const isMember = resolved !== undefined && walk.members.has(resolved);
            const isDestack = resolved !== undefined && existsSync(join(resolved, "destack.json"));
            // refuse a missing dependency the package cannot build without
            if (resolved === undefined && !isOptional) {
                throw new BuildError(
                    "BUILD_FAILED",
                    `missing dependency of ${name}: ${dependency}`,
                );
            }
            // build a Destack member as a unit
            else if (resolved !== undefined && isMember && isDestack) {
                requires.push((await Workspace.#visit(resolved, walk)).name);
            }
            // keep an npm package from outside the workspace, stored as npm units
            else if (resolved !== undefined && !isMember && !isDestack) {
                npm.set(dependency, resolved);
            }
        }
        const visited = { name, directory: from, packageJson, requires, npm };
        walk.units.set(from, visited);

        return visited;
    }

    /** Find the workspace a lockfile belongs to, past a link to it. */
    static async #locate(lockfile: BunLockfile): Promise<string> {
        return dirname(await realpath(join(lockfile.directory, "bun.lock")));
    }

    /** The store the workspace's units build into. */
    get #store(): LocalPackageStore {
        return this.#compilers.store;
    }

    /** Build the root on its dependencies' builds with the given outputs, reusing stored units. */
    async build(request: WorkspaceBuildOptions): Promise<PackageBuild> {
        // build the root's dependency units, the root's commit its own
        const { signal, timeout, versions, history } = request;
        const shared = {
            ...(signal === undefined ? {} : { signal }),
            ...(timeout === undefined ? {} : { timeout }),
            ...(versions === undefined ? {} : { versions }),
            ...(history === undefined ? {} : { history }),
        };
        const { dependencies } = await this.#resolve(this.root, await this.#storeEach(shared));

        // restore the root from the store, else build it
        const options = this.#options(this.root, request, dependencies);
        const key = await this.#key(this.root, options);
        const cached = await this.#store.cached(key);
        const built =
            cached?.kind === "build"
                ? await PackageBuild.restore(
                      this.#store,
                      cached.manifest,
                      await mkdtemp(join(tmpdir(), "destack-build-")),
                  )
                : await this.#compile(this.root, options);

        // key the build, and root it as the root's latest
        const manifest = await PackageStore.digest(built.manifest);
        await this.#store.cache(key, { kind: "build", manifest });
        await this.#rootUnit(this.root, manifest, dependencies);

        return built;
    }

    /** Inspect the root against its dependencies' builds, reusing stored units. */
    async inspect(request: Omit<InspectOptions, "directory">): Promise<PackageInspection> {
        return await this.#against(
            async (compiler, dependencies) =>
                await compiler.inspect({
                    ...request,
                    directory: this.root.directory,
                    dependencies,
                }),
        );
    }

    /** Index the root's code against its dependencies' builds, reusing stored units. */
    async index(request: Omit<InspectOptions, "directory">): Promise<graph.Document[]> {
        return await this.#against(
            async (compiler, dependencies) =>
                await compiler.index({ ...request, directory: this.root.directory, dependencies }),
        );
    }

    /** Store every unit before the root for all its runtimes, answering the root's dependencies. */
    async storeDependencies(
        shared: WorkspaceUnitOptions,
    ): Promise<Record<string, DependencyResolution>> {
        const { dependencies } = await this.#resolve(this.root, await this.#storeEach(shared));

        return dependencies;
    }

    /** Store every unit's build, by unit name in build order, the root last. */
    async storeUnits(shared: WorkspaceUnitOptions): Promise<ReadonlyMap<string, DependencyBuild>> {
        const stored = await this.#storeEach(shared, this.units);

        return new Map([...stored].map(([name, unit]) => [name, unit.build]));
    }

    /** Run work with the root's compiler against the builds of its dependencies. */
    async #against<Result>(
        work: (
            compiler: Compiler,
            dependencies: Record<string, DependencyResolution>,
        ) => Promise<Result>,
    ): Promise<Result> {
        const { dependencies } = await this.#resolve(this.root, await this.#storeEach({}));

        return await this.#compilers.use(this.root.directory, (compiler) =>
            work(compiler, dependencies),
        );
    }

    /** Store some units, by default those before the root, each after the units it requires. */
    async #storeEach(
        shared: WorkspaceUnitOptions,
        units: readonly WorkspaceUnit[] = this.units.filter((unit) => unit !== this.root),
    ): Promise<ReadonlyMap<string, StoredUnit>> {
        // start every unit at once after the units it requires, as the pool leases compilers in turn
        const started = new Map<string, Promise<StoredUnit>>();
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
        started: ReadonlyMap<string, Promise<StoredUnit>>,
        shared: WorkspaceUnitOptions,
    ): Promise<StoredUnit> {
        // wait for the units it reaches, then resolve its dependencies on their builds
        const reached = new Map<string, StoredUnit>();
        for (const name of this.#reach(unit)) {
            reached.set(name, await found(started, name));
        }
        const { dependencies, releases } = await this.#resolve(unit, reached);

        // build every output the unit ships, its dependencies kept external but in a browser output
        const outputs = outputsOf(unit.packageJson);
        const options = this.#options(unit, { ...shared, outputs }, dependencies);
        const key = await this.#key(unit, options);

        // take its stored build, building it once when missing
        const manifest = await this.#store.cachedBuild(key, () => this.#put(unit, options));
        const built = PackageManifest.package(await this.#store.get(manifest));

        // root the build as the unit's latest
        await this.#rootUnit(unit, manifest, dependencies);

        return { build: { kind: "destack", package: built, manifest }, releases };
    }

    /** List the units a unit requires, and theirs, each once. */
    #reach(unit: WorkspaceUnit): ReadonlySet<string> {
        // walk the requirements once each
        const reached = new Set<string>();
        const pending = [...unit.requires];
        for (let name = pending.pop(); name !== undefined; name = pending.pop()) {
            if (!reached.has(name)) {
                reached.add(name);
                pending.push(...found(this.#named, name).requires);
            }
        }

        return reached;
    }

    /** Resolve a unit's dependencies on the stored units it reaches, with its npm releases. */
    async #resolve(
        unit: WorkspaceUnit,
        stored: ReadonlyMap<string, StoredUnit>,
    ): Promise<{
        readonly dependencies: Record<string, DependencyResolution>;
        readonly releases: Record<string, NpmDependency>;
    }> {
        // take the builds of the units it reaches, as kinds may come from any, with their npm releases
        const dependencies: Record<string, DependencyResolution> = {};
        for (const name of this.#reach(unit)) {
            const reached = found(stored, name);
            dependencies[name] = reached.build;
            Object.assign(dependencies, reached.releases);
        }

        // store its own npm dependencies, by the names it imports them by and by release
        const { named, releases } = await this.#storeNpm(unit);

        return { dependencies: { ...dependencies, ...releases, ...named }, releases };
    }

    /** Store the npm units a unit's npm dependencies reach, by import name and by release. */
    async #storeNpm(unit: WorkspaceUnit): Promise<{
        readonly named: Record<string, NpmDependency>;
        readonly releases: Record<string, NpmDependency>;
    }> {
        // store each npm dependency's unit, converting in the unit's compiler
        const named: Record<string, NpmDependency> = {};
        const releases: Record<string, NpmDependency> = {};
        const convert: NpmConverter = (request) =>
            this.#compilers.use(unit.directory, (compiler) => compiler.convert(request));
        const sorted = [...unit.npm].toSorted(([left], [right]) => Text.compare(left, right));
        for (const [name, directory] of sorted) {
            const opened = await this.#npm.open(directory);
            named[name] = await this.#npm.store(opened, convert);

            // record every unit it reaches by release
            for (const reached of opened.closure()) {
                const { name: released, version } = reached.release.package;
                releases[`${released}@${version}`] = await this.#npm.store(reached, convert);
            }
        }

        return { named, releases };
    }

    /** Build options of a unit on its dependencies, with its own version and history. */
    #options(
        unit: WorkspaceUnit,
        request: WorkspaceBuildOptions,
        dependencies: Record<string, DependencyResolution>,
    ): PackageBuildOptions {
        const { history, ...rest } = request;
        const published = history?.[unit.name];

        return {
            ...rest,
            directory: unit.directory,
            dependencies,
            ...(published === undefined ? {} : { history: published }),
        };
    }

    /** Key a unit's build by its checkout, dependencies, toolchain, configuration and release. */
    async #key(
        unit: WorkspaceUnit,
        options: Omit<PackageBuildOptions, "signal" | "timeout">,
    ): Promise<string> {
        return await Digest.json({
            kind: "unit",
            toolchain: this.#toolchain,
            checkout: await this.#checkout.identify(unit.directory),
            dependencies: options.dependencies,
            outputs: options.outputs,
            configuration:
                options.configuration === undefined
                    ? null
                    : await this.#identities.file(resolve(unit.directory, options.configuration)),
            version: options.versions?.[unit.name] ?? null,
            history: options.history ?? null,
            commit: options.commit ?? null,
        });
    }

    /** Root a unit's latest build and its dependencies' builds by its directory. */
    async #rootUnit(
        unit: WorkspaceUnit,
        manifest: string,
        dependencies: Readonly<Record<string, DependencyResolution>>,
    ): Promise<void> {
        const released = Object.values(dependencies).flatMap((dependency) =>
            dependency.kind === "source" ? [] : [dependency.manifest],
        );
        await this.#store.rootDirectory(unit.directory, new Set([manifest, ...released]));
    }

    /** Compile a unit, which puts it into the store, answering its manifest's digest. */
    async #put(unit: WorkspaceUnit, options: PackageBuildOptions): Promise<string> {
        await using build = await this.#compile(unit, options);

        return await PackageStore.digest(build.manifest);
    }

    /** Compile a unit with its compiler, which stores the build. */
    async #compile(unit: WorkspaceUnit, options: PackageBuildOptions): Promise<PackageBuild> {
        return await this.#compilers
            .use(unit.directory, (compiler) => compiler.build(options))
            .catch((cause: unknown) => {
                throw new BuildError("BUILD_FAILED", `building ${unit.name} failed`, { cause });
            });
    }
}
