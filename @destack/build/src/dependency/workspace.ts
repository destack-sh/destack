import type { EvaluatedDeclaration } from "../declaration/evaluation.ts";
import { dirname, join, resolve } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import type { BunLockfile } from "@destack/check/bun";
import { DomainError } from "@destack/error";
import { type DependencyResolution, PackageJson, type graph } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { PackageLocator } from "@destack/package/locator";
import type { Runtime } from "@destack/package/runtime";
import { Digest, found, nth } from "@destack/schema";
import { Checkout, Identities } from "../cache/index.ts";
import { NpmInstallation } from "./npm.ts";
import { outputsOf, type PackageBuildOptions, PackageBuild } from "../build/build.ts";
import type { Compiler } from "../build/compiler.ts";
import type { CompilerPool } from "../build/pool.ts";
import type { InspectOptions } from "../inspection/inspection.ts";
import type { PackageInspection } from "@destack/package/code";
import { BuildError } from "../error/index.ts";
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

/** A build of a workspace's root, sharing its deadline and versions with its units. */
export type WorkspaceBuildOptions = Omit<PackageBuildOptions, "directory" | "dependencies">;

/** What a workspace's units share when stored: cancellation, deadline and versions. */
export type WorkspaceUnitOptions = Pick<WorkspaceBuildOptions, "signal" | "timeout" | "versions">;

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
    /** The npm packages the lockfile locks, each stored once as a unit. */
    readonly #npm: NpmInstallation;

    /** Store a closure's units in build order, the root last, with what their keys read. */
    private constructor(
        units: readonly WorkspaceUnit[],
        compilers: WorkspaceCompilers,
        identities: Identities,
        toolchain: Digest,
        checkout: Checkout,
        npm: NpmInstallation,
    ) {
        // store the units and the compilers
        this.units = units;
        this.root = nth(units, units.length - 1);
        this.#named = new Map(units.map((unit) => [unit.name, unit]));
        this.#compilers = compilers;

        // store what the units' keys read
        this.#identities = identities;
        this.#toolchain = toolchain;
        this.#checkout = checkout;
        this.#npm = npm;
    }

    /** Read the closure of Destack workspace members a package requires, in build order. */
    static async open(directory: string, compilers: WorkspaceCompilers): Promise<Workspace> {
        // read the workspace's members by their real directories, refusing a missing one
        const { environment } = compilers.store;
        const { files } = environment;
        const lockfile = await readLockfile(directory, files);
        const workspace = await Workspace.#locate(lockfile, files);
        const members = new Set(
            await Promise.all(
                Object.entries(lockfile.workspaces).map(async ([path, member]) => {
                    const location = join(workspace, path);

                    return await files.realPath(location).catch((cause: unknown) => {
                        if (!FileSystemError.is(cause, "NOT_FOUND")) {
                            throw cause;
                        }
                        throw BuildError.of(
                            "MEMBER_UNKNOWN",
                            `the workspace member ${member.name ?? path} is missing at ${location}`,
                            { cause },
                        );
                    });
                }),
            ),
        );

        // walk the Destack members the root requires through their installed links
        const units = new Map<string, WorkspaceUnit>();
        const locator = new PackageLocator(files);
        const walk = { locator, files, members, units, visiting: new Set<string>() };
        await Workspace.#visit(await files.realPath(directory), walk);

        // read the toolchain, the checkout and the locked npm releases the units' keys read
        const identities = Identities.of(environment);
        const toolchain = await identities.toolchain();
        const checkout = await Checkout.list(workspace, environment);
        const locked = readLockedReleases(lockfile);
        const conversion = await identities.conversion();
        const npm = new NpmInstallation(locked, compilers.store, conversion, environment);

        return new Workspace([...units.values()], compilers, identities, toolchain, checkout, npm);
    }

    /** Visit a member and the Destack members it requires, each before the members requiring it. */
    static async #visit(
        from: string,
        walk: {
            readonly locator: PackageLocator;
            readonly files: FileSystem;
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
            throw BuildError.of("DEPENDENCY_INVALID", `workspace dependency cycle through ${from}`);
        }
        walk.visiting.add(from);
        const packageJson = await readPackageJson(from, walk.files);
        const name = packageJson.package.name;

        // split each runtime or listed dependency once into units and npm packages, leaving the rest to the compiler
        const listed = packageJson.definition.extensions ?? [];
        const requires: string[] = [];
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
            const isMember = resolved !== undefined && walk.members.has(resolved);
            const isDestack =
                resolved !== undefined && (await walk.files.exists(join(resolved, "destack.json")));
            // refuse a missing dependency the package cannot build without
            if (resolved === undefined && !isOptional) {
                throw BuildError.of(
                    "DEPENDENCY_UNRESOLVED",
                    `missing dependency of ${name}: ${dependency}`,
                );
            }
            // build a Destack member as a unit
            else if (resolved !== undefined && isMember && isDestack) {
                requires.push((await Workspace.#visit(resolved, walk)).name);
            }
            // take an npm package from outside the workspace, stored as npm units
            else if (resolved !== undefined && !isMember && !isDestack) {
                npm.set(dependency, resolved);
            }
        }
        const visited = { name, directory: from, packageJson, requires, npm };
        walk.units.set(from, visited);

        return visited;
    }

    /** Find the workspace a lockfile belongs to, past a link to it. */
    static async #locate(lockfile: BunLockfile, files: FileSystem): Promise<string> {
        return dirname(await files.realPath(join(lockfile.directory, "bun.lock")));
    }

    /** The store the workspace's units build into. */
    get #store(): LocalPackageStore {
        return this.#compilers.store;
    }

    /** Build the root on its dependencies' builds with the given outputs, reusing stored units. */
    async build(request: WorkspaceBuildOptions): Promise<PackageBuild> {
        // build the root's dependency units, the root's commit separate
        const { signal, timeout, versions } = request;
        const shared = {
            ...(signal === undefined ? {} : { signal }),
            ...(timeout === undefined ? {} : { timeout }),
            ...(versions === undefined ? {} : { versions }),
        };

        // build the dependencies, each with every output it ships
        const stored = await this.#storeEach(shared, this.#dependencies);
        const dependencies = this.#resolve(this.root, stored);

        // take the root's stored build, relinking or compiling it once when missing, then restore it
        const options = this.#options(this.root, request, dependencies);
        const key = await this.#key(this.root, options);
        const manifest = await this.#store.cachedBuild(key, () => this.#put(this.root, options));
        await this.#rootUnit(this.root, manifest, dependencies);

        return await PackageBuild.restore(
            this.#store,
            manifest,
            await this.#store.environment.files.makeTemporaryDirectory("destack-build-"),
        );
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
                await compiler.index({ ...request, directory: this.root.directory, dependencies }),
        );
    }

    /** Store every unit before the root for all its runtimes, answering the root's dependencies. */
    async storeDependencies(
        shared: WorkspaceUnitOptions,
    ): Promise<Record<string, DependencyResolution>> {
        const stored = await this.#storeEach(shared, this.#dependencies);
        const dependencies = this.#resolve(this.root, stored);

        return dependencies;
    }

    /** Store every unit's build, by unit name in build order, the root last. */
    async storeUnits(shared: WorkspaceUnitOptions): Promise<ReadonlyMap<string, DependencyBuild>> {
        const stored = await this.#storeEach(shared, this.units);

        return stored;
    }

    /** Run work with the root's compiler against the builds of its dependencies. */
    async #against<Result>(
        runtime: Runtime,
        work: (
            compiler: Compiler,
            dependencies: Record<string, DependencyResolution>,
        ) => Promise<Result>,
    ): Promise<Result> {
        const stored = await this.#storeEach({}, this.#dependencies);
        const dependencies = this.#resolve(this.root, stored);

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
        // wait for the units it requires, then resolve its dependencies on their builds
        const required = new Map<string, DependencyBuild>();
        for (const name of this.#requirements(unit)) {
            required.set(name, await found(started, name));
        }
        const dependencies = this.#resolve(unit, required);

        // build every output the unit ships
        const outputs = outputsOf(unit.packageJson);
        const options = this.#options(unit, { ...shared, outputs }, dependencies);
        const key = await this.#key(unit, options);

        // take its stored build, building it once when missing
        const manifest = await this.#store.cachedBuild(key, () => this.#put(unit, options));
        const built = PackageManifest.package(await this.#store.get(manifest));

        // root the build as the unit's latest
        await this.#rootUnit(unit, manifest, dependencies);

        return { kind: "destack", package: built, manifest };
    }

    /** List the units a unit requires, and theirs, each once. */
    #requirements(unit: WorkspaceUnit): ReadonlySet<string> {
        // walk the requirements once each
        const closure = new Set<string>();
        const pending = [...unit.requires];
        for (let name = pending.pop(); name !== undefined; name = pending.pop()) {
            if (!closure.has(name)) {
                closure.add(name);
                pending.push(...found(this.#named, name).requires);
            }
        }

        return closure;
    }

    /** Resolve a unit's dependencies on the stored builds of every unit it requires. */
    #resolve(
        unit: WorkspaceUnit,
        stored: ReadonlyMap<string, DependencyBuild>,
    ): Record<string, DependencyResolution> {
        return Object.fromEntries(
            [...this.#requirements(unit)].map((name) => [name, found(stored, name)]),
        );
    }

    /** List the npm releases a unit's npm dependencies lock, which its build's pins follow. */
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

    /** Build options of a unit on its dependencies. */
    #options(
        unit: WorkspaceUnit,
        request: WorkspaceBuildOptions,
        dependencies: Record<string, DependencyResolution>,
    ): PackageBuildOptions {
        return { ...request, directory: unit.directory, dependencies };
    }

    /** Key a unit's build by its sources and its dependencies' exact builds. */
    async #key(
        unit: WorkspaceUnit,
        options: Omit<PackageBuildOptions, "signal" | "timeout">,
    ): Promise<string> {
        return await Digest.json(
            {
                kind: "unit",
                ...(await this.#sources(unit, options)),
                dependencies: options.dependencies,
            },
            this.#store.environment.crypto,
        );
    }

    /** Key a unit's compilation by its sources, dependency interfaces and declarations. */
    async #compileKey(
        unit: WorkspaceUnit,
        options: Omit<PackageBuildOptions, "signal" | "timeout">,
        declarations: readonly EvaluatedDeclaration[],
    ): Promise<string> {
        // read each Destack dependency's interface, and each other dependency whole
        const interfaces = await Promise.all(
            Object.entries(options.dependencies).map(
                async ([name, dependency]): Promise<readonly [string, unknown]> => [
                    name,
                    dependency.kind === "destack"
                        ? (await this.#store.get(dependency.manifest)).interface
                        : dependency,
                ],
            ),
        );

        return await Digest.json(
            {
                kind: "compile",
                ...(await this.#sources(unit, options)),
                interfaces: Object.fromEntries(interfaces),
                declarations,
            },
            this.#store.environment.crypto,
        );
    }

    /** Read what a unit's build reads of its own: checkout, toolchain, outputs and release. */
    async #sources(
        unit: WorkspaceUnit,
        options: Omit<PackageBuildOptions, "signal" | "timeout">,
    ): Promise<Record<string, unknown>> {
        return {
            toolchain: this.#toolchain,
            checkout: await this.#checkout.identify(unit.directory),
            outputs: options.outputs,
            configuration:
                options.configuration === undefined
                    ? null
                    : await this.#identities.file(resolve(unit.directory, options.configuration)),
            version: options.versions?.[unit.name] ?? null,
            commit: options.commit ?? null,
            npm: await this.#npmReleases(unit),
        };
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

    /** Build a unit, relinking an earlier build of the same compile inputs. */
    async #put(unit: WorkspaceUnit, options: PackageBuildOptions): Promise<Digest> {
        // evaluate the unit's declarations on its dependencies, which run their code
        const declarations = await this.#use(unit, (compiler) => compiler.evaluate(options));

        // relink a build of the same compile inputs under the new pins
        const key = await this.#compileKey(unit, options, declarations);
        const cached = await this.#store.cached(key);
        if (cached?.kind === "build") {
            return await this.#store.relink(cached.manifest, options.dependencies);
        }

        // else compile it, and key the build by its compile inputs
        await using build = await this.#use(unit, (compiler) =>
            compiler.compile(options, declarations),
        );
        const manifest = await PackageStore.digest(build.manifest, this.#store.environment.crypto);
        await this.#store.cache(key, { kind: "build", manifest });

        return manifest;
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
                throw BuildError.of("INVARIANT_VIOLATED", `building ${unit.name} failed`, {
                    cause,
                });
            }
        });
    }
}
