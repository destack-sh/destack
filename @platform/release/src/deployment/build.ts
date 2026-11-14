import { join } from "node:path";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition } from "@destack/account/server";
import * as stacks from "@platform/stack";
import { RESIDENCIES } from "@platform/stack";
import { SERVER_EXPORT } from "@destack/object";
import { InstallationDefinition } from "@destack/stack/declare";
import {
    CompilerPool,
    LocalBuildStore,
    Workspace,
    type WorkspaceUnitOptions,
} from "@destack/build";
import { COMPUTER_RUNTIMES } from "@destack/machine/computer";
import { CLOUD_RUNTIMES } from "@destack/machine/cloudflare";
import type { Environment } from "@destack/environment";
import {
    DISTRIBUTION_LAYER,
    DISTRIBUTION_MANIFEST,
    DistributionManifest,
    DistributionPlatform,
    PackageName,
} from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import {
    type Digest,
    found,
    type Identifier,
    type JsonValue,
    present,
    schema,
} from "@destack/schema";
import type { Registry } from "@destack/registry/client";
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

/** A forge a platform build releases into: its account and the registry calling it. */
export interface ReleaseForge {
    /** The publishing account. */
    readonly accountId: Identifier<"account">;
    /** The registry calling the account's forge. */
    readonly registry: Registry;
}

/** Open the forge of a handle's account, as a machine publishing in its residency's home. */
export type ForgeOpen = (handle: string) => Promise<ReleaseForge>;

/** The first-party releases a universe's homes run and its forge publishes. */
export class PlatformBuild implements AsyncDisposable {
    /** The store keeping every build and lock, locked to this process until disposed. */
    readonly store: LocalBuildStore;
    /** The releases of the stacks' closure and the published packages, by package name. */
    readonly releases: ReadonlyMap<string, BuildReader>;
    /** The lock of each release: its build and every build it runs with, by package name. */
    readonly locks: ReadonlyMap<string, Digest>;

    /** Keep built releases and their locks. */
    private constructor(
        store: LocalBuildStore,
        releases: ReadonlyMap<string, BuildReader>,
        locks: ReadonlyMap<string, Digest>,
    ) {
        this.store = store;
        this.releases = releases;
        this.locks = locks;
    }

    /** Build the stacks' closure and a checkout's published packages into its store. */
    static async build(
        root: string,
        host: Environment<"files" | "process" | "host">,
        options: Pick<WorkspaceUnitOptions, "isPrerelease"> = {},
    ): Promise<PlatformBuild> {
        // open the store and the compilers, closing the store when a build fails
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await LocalBuildStore.open(join(root, RELEASES_DIRECTORY), host));
        await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());

        // read the workspace once, and the packages of the stacks and of the desktop, which runs the machine
        const workspace = await Workspace.open(root, compilers);
        const packages = [
            await workspace.package(join(root, STACK)),
            await workspace.package(join(root, DESKTOP)),
        ];

        // build and lock each unit, a unit both packages share once
        const releases = new Map<string, BuildReader>();
        const locks = new Map<string, Digest>();
        for (const unitPackage of packages) {
            for (const [name, { build, lock }] of await unitPackage.lockUnits(options)) {
                releases.set(name, await store.contents(build.manifest));
                locks.set(name, lock);
            }
        }

        // hand the store to the build, which closes it
        opened.move();

        return new PlatformBuild(store, releases, locks);
    }

    /** Release each lock under a tag into its account's forge, a dependency layer at a time. */
    async release(
        tag: string,
        open: ForgeOpen,
        released: (reader: BuildReader) => Promise<void>,
    ): Promise<void> {
        // release each handle's locks at once into its account's forge, after the handles they require
        for (const { handle, names } of PlatformBuild.#runs(this.releases)) {
            const { accountId, registry } = await open(handle);
            const locks = names.map((name) => found(this.locks, name));
            const releases = await registry.release(this.store, locks, accountId, tag);
            for (const each of releases) {
                await released(found(this.releases, each.package.name));
            }
        }
    }

    /** Group releases by handle, each handle after the handles its releases require. */
    static #runs(
        releases: ReadonlyMap<string, BuildReader>,
    ): { readonly handle: string; readonly names: readonly string[] }[] {
        // group the names by handle, with the other handles each handle's releases require
        const names = new Map<string, string[]>();
        const requires = new Map<string, Set<string>>();
        for (const name of releases.keys()) {
            const { handle } = PackageName.split(name);
            const required = Object.keys(found(releases, name).manifest.requires)
                .filter((dependency) => releases.has(dependency))
                .map((dependency) => PackageName.split(dependency).handle)
                .filter((other) => other !== handle);
            names.set(handle, [...(names.get(handle) ?? []), name]);
            requires.set(handle, new Set([...(requires.get(handle) ?? []), ...required]));
        }

        // visit each handle's required handles before it, refusing handles that require each other
        const runs: { readonly handle: string; readonly names: readonly string[] }[] = [];
        const visited = new Set<string>();
        const visiting = new Set<string>();
        const visit = (handle: string) => {
            if (visited.has(handle)) {
                return;
            }
            if (visiting.has(handle)) {
                throw new Error(`releases of '@${handle}' require releases that require them back`);
            }
            visiting.add(handle);
            for (const other of requires.get(handle) ?? []) {
                visit(other);
            }
            visiting.delete(handle);
            visited.add(handle);
            runs.push({ handle, names: found(names, handle) });
        };
        for (const handle of names.keys()) {
            visit(handle);
        }

        return runs;
    }

    /** Close the builds' store, releasing its lock. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.store[Symbol.asyncDispose]();
    }

    /** Every release the build keeps, which machines ship and seed their pull caches with. */
    get readers(): readonly BuildReader[] {
        return [...this.releases.values()];
    }

    /** List every release the build keeps with its role and its lock on a platform. */
    distribution(platform: DistributionPlatform): DistributionManifest {
        const releases = [...this.releases].map(([name, reader]) => ({
            package: PackageManifest.package(reader.manifest),
            role: reader.manifest.role,
            locks: { [platform]: found(this.locks, name) },
        }));

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
        await this.store.writeLayer(
            [...this.locks.values()],
            CLOUD_RUNTIMES,
            directory,
            "addressed",
        );
    }

    /** Write the manifest of the releases computers on the host run to a directory. */
    async writeDistribution(directory: string): Promise<void> {
        // write the host's manifest into the directory, created if new
        const { files, host } = this.store.environment;
        const distribution = this.distribution(DistributionPlatform.of(host));
        const text = JSON.stringify(DistributionManifest.parse(distribution));
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(join(directory, DISTRIBUTION_MANIFEST), text);
    }

    /** Write the releases with what computers run as a layer, as local cloud machines read it. */
    async writeLayer(directory: string): Promise<void> {
        await this.store.writeLayer(
            [...this.locks.values()],
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

    /** Select an export of the stack package's built release with its settings, as a home does. */
    released(
        exported: keyof typeof stacks,
        settings: Readonly<Record<string, JsonValue>>,
    ): StackRelease {
        // select the stack package's built release by its version
        const stack = present(this.releases.get(STACK), `the release of ${STACK}`);
        const { id, version } = PackageManifest.package(stack.manifest);

        return {
            packageId: id,
            selection: { kind: "release", version },
            export: exported,
            settings: { ...settings },
        };
    }
}
