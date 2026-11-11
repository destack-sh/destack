import { join } from "node:path";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition } from "@destack/account/server";
import * as stacks from "@platform/stack";
import { RESIDENCIES } from "@platform/stack";
import { SERVER_EXPORT } from "@destack/object";
import { InstallationDefinition } from "@destack/stack/declare";
import { CompilerPool, LocalBuildStore, Workspace } from "@destack/build";
import { BuildStore } from "@destack/build/store";
import { COMPUTER_RUNTIMES } from "@destack/machine/computer";
import { CLOUD_RUNTIMES } from "@destack/machine/cloudflare";
import type { Environment } from "@destack/environment";
import {
    DISTRIBUTION_LAYER,
    DISTRIBUTION_MANIFEST,
    DistributionManifest,
    type Package,
} from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { type Digest, type JsonValue, present, schema, type Version } from "@destack/schema";
import { Toolchain } from "@destack/check/toolchain";

/** The package declaring the homes' stacks, by its directory below the checkout's root. */
const STACK = "@platform/stack";

/** The export declaring the stack of each kind of space people create. */
const EXPORTS = {
    home: "HOME_STACK",
    space: "SPACE_STACK",
} as const satisfies Record<keyof PlatformDefinition["stacks"], keyof typeof stacks>;

/** The exports declaring each residency home's stack, the same on every stage. */
const STACK_EXPORTS = RESIDENCIES.map(
    (residency) => residency.stack,
) satisfies readonly (keyof typeof stacks)[];

/** A stack definition's installations, by name. */
const INSTALLATIONS = schema.record(schema.string(), InstallationDefinition);

/** The desktop machine's entry below the checkout, the runtime computers ship. */
export const DESKTOP_ENTRY = "@destack/desktop/src/machine.ts";

/** The desktop package, whose workspace builds the release of the program computers run. */
const DESKTOP = "@destack/desktop";

/** The checkout's store of first-party releases, which every stage and target shares. */
export const RELEASES_DIRECTORY = "target/releases";

/** A local build's versions. */
export interface LocalBuild {
    /** Read the versions a local build starts its packages at, by package name. */
    readonly versions: (packages: readonly Package[]) => Readonly<Record<string, Version>>;
    /** Read the next versions for the packages a build changed, none once nothing changed. */
    readonly settle: (
        built: ReadonlyMap<string, Digest>,
        versions: Readonly<Record<string, Version>>,
    ) => Readonly<Record<string, Version>> | undefined;
}

/** The first-party releases a universe's homes run and its forge publishes. */
export class PlatformBuild implements AsyncDisposable {
    /** The store keeping every build, locked to this process until disposed. */
    readonly store: LocalBuildStore;
    /** The releases of the stacks' closure and the published packages, by package name. */
    readonly releases: ReadonlyMap<string, BuildReader>;

    /** Keep built releases. */
    private constructor(store: LocalBuildStore, releases: ReadonlyMap<string, BuildReader>) {
        this.store = store;
        this.releases = releases;
    }

    /** Build the stacks' closure and the published packages of a checkout into its store. */
    static async build(
        root: string,
        host: Environment<"files" | "process" | "host">,
        local?: LocalBuild,
    ): Promise<PlatformBuild> {
        // open the store and the compilers, closing the store when a build fails
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await LocalBuildStore.open(join(root, RELEASES_DIRECTORY), host));
        await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());

        // read the workspaces of the stacks and of the desktop, which runs the machine
        const workspaces = [
            await Workspace.open(join(root, STACK), compilers),
            await Workspace.open(join(root, DESKTOP), compilers),
        ];

        // start a local build's versions over every unit's package
        const units = new Map(
            workspaces.flatMap((workspace) => workspace.units).map((unit) => [unit.name, unit]),
        );
        const packages = [...units.values()].map((unit) => unit.packageJson.package);
        let versions = local?.versions(packages);

        // store every unit's build, the units the workspaces share once, until its versions settle
        const releases = new Map<string, BuildReader>();
        for (;;) {
            const built = new Map<string, Digest>();
            for (const workspace of workspaces) {
                const shared = versions === undefined ? {} : { versions };
                for (const [name, build] of await workspace.storeUnits(shared)) {
                    releases.set(name, await store.contents(build.manifest));
                    built.set(name, build.manifest);
                }
            }
            const next = versions === undefined ? undefined : local?.settle(built, versions);
            if (next === undefined) {
                break;
            }
            versions = next;
        }

        // hand the store to the build, which closes it
        opened.move();

        return new PlatformBuild(store, releases);
    }

    /** Close the builds' store, releasing its lock. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.store[Symbol.asyncDispose]();
    }

    /** Every release the build keeps, which machines ship and seed their pull caches with. */
    get readers(): readonly BuildReader[] {
        return [...this.releases.values()];
    }

    /** List every release the build keeps with its role, the distribution machines ship. */
    async distribution(): Promise<DistributionManifest> {
        const releases = await Promise.all(
            this.readers.map(async (reader) => ({
                package: PackageManifest.package(reader.manifest),
                manifest: await BuildStore.digest(reader.manifest, this.store.environment.crypto),
                role: reader.manifest.role,
            })),
        );

        return { releases };
    }

    /** Map each shipped package a machine runs in its process to its server's specifier. */
    serverBundle(): Readonly<Record<string, string>> {
        // take the packages the stacks install, each on the host or as a builtin machines link
        const declared = [...Object.values(EXPORTS), ...STACK_EXPORTS].map((name) => stacks[name]);
        const installations = declared.flatMap((stack) =>
            Object.values(INSTALLATIONS.parse(stack.definition["installations"] ?? {})),
        );
        const inProcess = new Set(
            installations.filter((each) => each.isolation === "host").map((each) => each.packageId),
        );
        const installed = new Set(installations.map((each) => each.packageId));

        // map each such release exporting a server, refusing one installed on the host without one
        const bundled: Record<string, string> = {};
        for (const { manifest } of this.readers) {
            const { id, name } = PackageManifest.package(manifest);
            const isServing = Object.values(manifest.outputs).some(
                (output) => output.exports[SERVER_EXPORT] !== undefined,
            );
            if (inProcess.has(id) && !isServing) {
                throw new Error(`'${name}' is installed on the host but exports no server`);
            } else if (
                isServing &&
                (inProcess.has(id) || (installed.has(id) && manifest.role === "builtin"))
            ) {
                bundled[name] = `${name}${SERVER_EXPORT.slice(1)}`;
            }
        }

        return bundled;
    }

    /** Write the releases with what cloud Workers run as their assets, a layer by digest. */
    async writeAssets(directory: string): Promise<void> {
        const distribution = await this.distribution();
        const manifests = distribution.releases.map((release) => release.manifest);
        await this.store.writeLayer(manifests, CLOUD_RUNTIMES, directory, "addressed");
    }

    /** Write the distribution's manifest, the releases computers pin, to a directory. */
    async writeDistribution(directory: string): Promise<void> {
        // write the manifest into the directory, created if new
        const { files } = this.store.environment;
        const text = JSON.stringify(DistributionManifest.parse(await this.distribution()));
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(join(directory, DISTRIBUTION_MANIFEST), text);
    }

    /** Write the releases with what computers run as a layer, as local cloud machines read it. */
    async writeLayer(directory: string): Promise<void> {
        const distribution = await this.distribution();
        const manifests = distribution.releases.map((release) => release.manifest);
        await this.store.writeLayer(
            manifests,
            COMPUTER_RUNTIMES,
            join(directory, DISTRIBUTION_LAYER),
            "linked",
        );
    }

    /** List the stacks spaces people create are born with, following the stack package's tag. */
    stacks(tag: string): PlatformDefinition["stacks"] {
        return {
            home: this.following(EXPORTS.home, tag),
            space: this.following(EXPORTS.space, tag),
        };
    }

    /** Follow an export of the stack package at a tag, as a space people create does. */
    following(exported: keyof typeof stacks, tag: string): StackRelease {
        const stack = present(this.releases.get(STACK), `the release of ${STACK}`);

        return {
            packageId: PackageManifest.package(stack.manifest).id,
            selection: { kind: "tag", tag },
            export: exported,
            settings: {},
        };
    }

    /** Pin an export of the stack package's built release with its settings, as a home does. */
    async pinned(
        exported: keyof typeof stacks,
        settings: Readonly<Record<string, JsonValue>>,
    ): Promise<StackRelease> {
        // pin the stack package's built release by its manifest
        const stack = present(this.releases.get(STACK), `the release of ${STACK}`);
        const { id, version } = PackageManifest.package(stack.manifest);
        const manifest = await BuildStore.digest(stack.manifest, this.store.environment.crypto);

        return {
            packageId: id,
            selection: { kind: "release", version, manifest },
            export: exported,
            settings: { ...settings },
        };
    }
}
