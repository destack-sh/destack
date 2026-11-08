import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { BuildStore } from "@destack/build/store";
import { Registry } from "@destack/registry/client";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import registryPackage from "@destack/registry/package";
import type { Machine } from "@destack/machine";
import { PackageName } from "@destack/package";
import { Digest, type Identifier, present, schema, Version } from "@destack/schema";
import { InstallationMount, PackageMount } from "@destack/object";
import { type ClientOf, createClient } from "@destack/object/client";
import { RequestId } from "@destack/sync";
import type { LocalBuild, PlatformBuild } from "../deployment/index.ts";

/** The file in a state directory recording its published local releases. */
const FILE = "publication.json";

/** A local version: its release and its `N` in `-dev.N`. */
const LOCAL_VERSION = /^(.+)-dev\.(\d+)$/u;

/** How long a started machine's directory replica may take to sync: a minute. */
export const DIRECTORY_SYNC_MILLISECONDS = 60_000;

/** How long a fresh residency home may take to serve its forge from cached builds, in ms. */
export const HOME_SERVE_MILLISECONDS = 30_000;

/** The local releases a local universe published, as its state directory records them. */
const Published = schema.object({
    /** The last release each package published, by package name. */
    packages: schema.record(
        schema.string(),
        schema.object({
            /** The release its local versions count from, such as `2026.9.0`. */
            release: schema.string().min(1),
            /** Its last local version's `N` in `-dev.N`. */
            sequence: schema.int().positive(),
            /** The manifest it published at that version. */
            manifest: Digest,
        }),
    ),
});
/** The local releases a local universe published. */
type Published = schema.Infer<typeof Published>;

/** A forge a publication releases into, with the registry taking its builds. */
export interface ReleaseForge {
    /** The forge's client. */
    readonly client: ClientOf<typeof registryPackage>;
    /** The publishing account's residency home, which serves the forge. */
    readonly home: Identifier<"space">;
    /** The publishing account. */
    readonly accountId: Identifier<"account">;
    /** The registry the forge receives builds at. */
    readonly registry: Registry;
}

/** Open the forge of a handle's account, as a machine publishing in its residency's home. */
export type ForgeOpen = (handle: string) => Promise<ReleaseForge>;

/** The local releases a local universe's state directory published into its forges. */
export class Publication {
    /** The state directory. */
    readonly #directory: string;
    /** The releases published so far. */
    readonly #published: Published;
    /** The files the record lives in. */
    readonly #files: FileSystem;

    /** Keep a state directory's published releases. */
    private constructor(directory: string, published: Published, files: FileSystem) {
        this.#directory = directory;
        this.#published = published;
        this.#files = files;
    }

    /** Read the local releases a state directory published, none in a new one. */
    static async read(directory: string, files: FileSystem): Promise<Publication> {
        // read the record, absent before the first publication
        const text = await files
            .readTextFile(join(directory, FILE))
            .catch((error: unknown) => FileSystemError.absent(error));
        const published = text === undefined ? { packages: {} } : Published.parse(JSON.parse(text));

        return new Publication(directory, published, files);
    }

    /** Plan a local build: each package at its last local version, moving on where it changed. */
    get build(): LocalBuild {
        const recorded = this.#published.packages;

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
                // move on each package whose build differs from the manifest published at its version
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

    /** Publish a local build's releases under a tag into the forges, and record them. */
    async publish(
        build: PlatformBuild,
        tag: string,
        open: ForgeOpen,
        print: (line: string) => void,
    ): Promise<Publication> {
        // release each build, recording the local version it landed at, so a run failing midway goes on from it
        const packages = { ...this.#published.packages };
        await Publication.release(build, tag, open, async (reader, digest) => {
            // print the release and record its local version
            const { name, version } = PackageManifest.package(reader.manifest);
            print(`${name}@${version}`);
            const [, release = version, sequence = "1"] = LOCAL_VERSION.exec(version) ?? [];
            packages[name] = { release, sequence: Number(sequence), manifest: digest };
            await this.#record({ packages });
        });

        return new Publication(this.#directory, { packages }, this.#files);
    }

    /** Release a build's releases under a tag into their accounts' forges, each the forge lacks. */
    static async release(
        build: PlatformBuild,
        tag: string,
        open: ForgeOpen,
        released: (reader: BuildReader, digest: Digest) => Promise<void>,
    ): Promise<void> {
        const forges = new Map<string, Promise<ReleaseForge>>();
        for (const reader of build.readers) {
            // open the forge of the release's account once per handle
            const { id, name, version } = PackageManifest.package(reader.manifest);
            const digest = await BuildStore.digest(reader.manifest, build.store.environment.crypto);
            const { handle } = PackageName.split(name);
            const opened = forges.get(handle) ?? open(handle);
            forges.set(handle, opened);
            const { client, home, accountId, registry } = await opened;
            const { environment } = build.store;

            // create the package once, and read the release the forge has at the version
            await client.package.create({
                spaceId: home,
                requestId: RequestId.create(environment),
                id,
                accountId,
                name: PackageName.split(name).local,
            });
            const listed = await client.release.list({
                spaceId: home,
                where: { parentId: id, version },
                limit: 1,
            });
            const [published] = listed.items;

            // push and release a version the forge lacks, under the tag
            if (published === undefined) {
                const manifest = await registry.push(build.store, digest);
                await client.release.create({
                    spaceId: home,
                    requestId: RequestId.create(environment),
                    parentId: id,
                    manifest,
                    tag,
                });
                await released(reader, digest);
            }
            // refuse a version the forge has from another build
            else if (published.manifest !== digest) {
                throw new Error(`${name}@${version} is published from another build`);
            }
            // point the tag at a version the forge has from this build
            else {
                await client.package.point({
                    spaceId: home,
                    id,
                    requestId: RequestId.create(environment),
                    name: tag,
                    version,
                });
            }
        }
    }

    /** Open the forges a machine reaches, as a machine in each residency home it publishes in. */
    static forges(machine: Machine, print: (line: string) => void): ForgeOpen {
        return async (handle) => {
            // resolve the handle in the machine's synced replica of the directory, and its home at the universe
            await machine.synced(machine.environment.clock.timeout(DIRECTORY_SYNC_MILLISECONDS));
            const { directory, environment } = machine;
            const accountId = present(await directory.account(handle), `@${handle}`);
            const located = await (
                await machine.accounts()
            ).residency.locate({
                account: accountId,
            });
            const home = schema.identifier("space").parse(located);

            // wait until the home serves its forge, printing the home's steps and refusing at the bound with the one it waits for
            const space = await machine.space(home);
            const waiting = new AbortController();
            void (async () => {
                for await (const { status, reason } of space.health.watch(waiting.signal)) {
                    print(`${space.health.name} ${status}: ${reason}`);
                }
            })();
            try {
                await space.waitServed(
                    FORGE_INSTALLATION,
                    environment.clock.timeout(HOME_SERVE_MILLISECONDS),
                );
            } finally {
                waiting.abort();
            }

            // call the forge below its installation in the home, with tokens bound to the home
            const fetch = machine.fetch(registryPackage.id, { partition: home });
            const where = await directory.locate(home, environment.clock.now());
            const origin = InstallationMount.url(where.url, home, FORGE_INSTALLATION);
            const url = `${origin}${PackageMount.path(registryPackage.id)}`;
            const client = createClient(registryPackage, { url, fetch }, environment);

            return {
                client,
                home,
                accountId,
                registry: new Registry(async () => ({ url, space: home, fetch }), environment),
            };
        };
    }

    /** Write the published releases into the state directory. */
    async #record(published: Published): Promise<void> {
        await this.#files.writeTextFile(
            join(this.#directory, FILE),
            `${JSON.stringify(published, null, 4)}\n`,
        );
    }
}
