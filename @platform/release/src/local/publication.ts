import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { BuildStore } from "@destack/build/store";
import { Registry } from "@destack/registry/client";
import { PackageName } from "@destack/package";
import { Digest, type Identifier, present, schema, Version } from "@destack/schema";
import type { LocalBuild, PlatformBuild } from "../deployment/index.ts";

/** The file in a state directory recording the local versions of its builds. */
const FILE = "publication.json";

/** A local version: its release and its `N` in `-dev.N`. */
const LOCAL_VERSION = /^(.+)-dev\.(\d+)$/u;

/** The local version a local universe gave each package's last build. */
const Assigned = schema.object({
    /** The last local version of each package, by package name. */
    packages: schema.record(
        schema.string(),
        schema.object({
            /** The release its local versions count from, such as `2026.9.0`. */
            release: schema.string().min(1),
            /** Its last local version's `N` in `-dev.N`. */
            sequence: schema.int().positive(),
            /** The manifest built at that version. */
            manifest: Digest,
        }),
    ),
});
/** The local version a local universe gave each package's last build. */
type Assigned = schema.Infer<typeof Assigned>;

/** A forge a publication releases into: its account and the registry calling it. */
export interface ReleaseForge {
    /** The publishing account. */
    readonly accountId: Identifier<"account">;
    /** The registry calling the account's forge. */
    readonly registry: Registry;
}

/** Open the forge of a handle's account, as a machine publishing in its residency's home. */
export type ForgeOpen = (handle: string) => Promise<ReleaseForge>;

/** The local versions a state directory gave its builds, and their release into its forges. */
export class Publication {
    /** The state directory. */
    readonly #directory: string;
    /** The local version of each package's last build. */
    readonly #assigned: Assigned;
    /** The files the record lives in. */
    readonly #files: FileSystem;

    /** Keep a state directory's local versions. */
    private constructor(directory: string, assigned: Assigned, files: FileSystem) {
        this.#directory = directory;
        this.#assigned = assigned;
        this.#files = files;
    }

    /** Read the local versions a state directory gave its builds, none in a new one. */
    static async read(directory: string, files: FileSystem): Promise<Publication> {
        // read the record, absent before the first publication
        const text = await files
            .readTextFile(join(directory, FILE))
            .catch((error: unknown) => FileSystemError.absent(error));
        const assigned = text === undefined ? { packages: {} } : Assigned.parse(JSON.parse(text));

        return new Publication(directory, assigned, files);
    }

    /** Plan a local build: each package at its last local version, moving on where it changed. */
    get build(): LocalBuild {
        const recorded = this.#assigned.packages;

        return {
            versions: (packages) =>
                Object.fromEntries(
                    packages.map(({ name, version }) => {
                        const last = recorded[name];
                        const sequence = last?.release === version ? last.sequence : 1;

                        return [name, Version.parse(`${version}-dev.${sequence}`)];
                    }),
                ),
            settle: (built, versions) => {
                // move on each package whose build differs from the manifest recorded at its version
                const next = { ...versions };
                let isSettled = true;
                for (const [name, manifest] of built) {
                    const last = recorded[name];
                    const version = present(versions[name], `the version of ${name}`);
                    if (
                        last !== undefined &&
                        `${last.release}-dev.${last.sequence}` === version &&
                        last.manifest !== manifest
                    ) {
                        next[name] = Version.parse(`${last.release}-dev.${last.sequence + 1}`);
                        isSettled = false;
                    }
                }

                return isSettled ? undefined : next;
            },
        };
    }

    /** Record each built release's local version and manifest. */
    async assign(build: PlatformBuild): Promise<void> {
        // record each release's version and manifest
        const packages = { ...this.#assigned.packages };
        for (const reader of build.readers) {
            const { name, version } = PackageManifest.package(reader.manifest);
            const [, release = version, sequence = "1"] = LOCAL_VERSION.exec(version) ?? [];
            const manifest = await BuildStore.digest(
                reader.manifest,
                build.store.environment.crypto,
            );
            packages[name] = { release, sequence: Number(sequence), manifest };
        }
        await this.#record({ packages });
    }

    /** Release a build's releases under a tag into their accounts' forges, each the forge lacks. */
    static async release(
        build: PlatformBuild,
        tag: string,
        open: ForgeOpen,
        released: (reader: BuildReader, digest: Digest) => Promise<void>,
    ): Promise<void> {
        // layer the builds once, each layer pinning only builds of earlier ones
        const { crypto } = build.store.environment;
        const readers = new Map<Digest, BuildReader>();
        for (const reader of build.readers) {
            readers.set(await BuildStore.digest(reader.manifest, crypto), reader);
        }
        const layers = await BuildStore.layers(build.store, [...readers.keys()]);

        // release each layer's builds at once into the forge of their account, opened once per handle
        const forges = new Map<string, Promise<ReleaseForge>>();
        const releaseOne = async (digest: Digest) => {
            // read the build and its account's forge, then release it
            const reader = present(
                readers.get(digest),
                `build ${digest} is a dependency outside the platform build`,
            );
            const { handle } = PackageName.split(PackageManifest.package(reader.manifest).name);
            const opened = forges.get(handle) ?? open(handle);
            forges.set(handle, opened);
            const { accountId, registry } = await opened;
            if ((await registry.release(build.store, digest, accountId, tag)) === "released") {
                await released(reader, digest);
            }
        };
        for (const layer of layers) {
            await Promise.all(layer.map(releaseOne));
        }
    }

    /** Write the local versions into the state directory. */
    async #record(assigned: Assigned): Promise<void> {
        await this.#files.mkdir(this.#directory, { recursive: true });
        await this.#files.writeTextFile(
            join(this.#directory, FILE),
            `${JSON.stringify(assigned, null, 4)}\n`,
        );
    }
}
