import { PackageManifest } from "@destack/package/manifest";
import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { BuildStore } from "@destack/build/store";
import { Registry } from "@destack/registry/client";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import registryPackage from "@destack/registry/package";
import type { Machine } from "@destack/machine";
import { PackageName } from "@destack/package";
import {
    Digest,
    type Identifier,
    type UuidEnvironment,
    present,
    schema,
    Version,
} from "@destack/schema";
import { InstallationMount, PackageMount } from "@destack/object";
import { createClient, type ClientOf } from "@destack/object/client";
import { RequestId } from "@destack/sync";
import type { LocalBuild, PlatformBuild } from "../deployment/index.ts";

/** The file in a state directory recording its published local releases. */
const FILE = "publication.json";

/** A local version: its release and its `N` in `-dev.N`. */
const LOCAL_VERSION = /^(.+)-dev\.(\d+)$/u;

/** How long a started machine's directory replica may take to sync: a minute. */
const DIRECTORY_SYNC_MILLISECONDS = 60_000;

/** How long a fresh residency home may take to serve its forge from cached builds, in ms. */
const HOME_SERVE_MILLISECONDS = 30_000;

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

/** The forge of a residency home: its service, and its registry receiving builds. */
interface Forge {
    /** The forge service, as the publishing machine in the home. */
    readonly service: ClientOf<typeof registryPackage>;
    /** The registry the forge receives builds at. */
    readonly registry: Registry;
}

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
        machine: Machine,
        print: (line: string) => void,
    ): Promise<Publication> {
        // resolve handles in the machine's synced replica of the directory, and homes at the universe
        await machine.synced(machine.environment.clock.timeout(DIRECTORY_SYNC_MILLISECONDS));
        const { directory } = machine;
        const residencies = (await machine.accounts()).residency;
        const forges = new Map<string, Forge>();

        // publish each release into the forge of its scope's account, once per manifest
        const packages = { ...this.#published.packages };
        for (const reader of build.readers) {
            // leave a release this universe published already
            const { id, name, version } = PackageManifest.package(reader.manifest);
            const digest = await BuildStore.digest(reader.manifest, machine.environment.crypto);
            if (packages[name]?.manifest === digest) {
                continue;
            }

            // find the account of the release's scope and the forge of its residency's home
            const parsed = PackageName.split(name);
            const accountId = present(await directory.account(parsed.handle), `@${parsed.handle}`);
            const home = await residencies.locate({ account: accountId });
            const forge =
                forges.get(home) ??
                (await Publication.#forge(machine, schema.identifier("space").parse(home), print));
            forges.set(home, forge);

            // create the package once, push its build and release it under the tag
            await Publication.#createPackage(
                forge.service,
                home,
                accountId,
                { id, local: parsed.local },
                machine.environment,
            );
            const manifest = await forge.registry.push(build.store, digest);
            await forge.service.release.create({
                spaceId: home,
                requestId: RequestId.create(machine.environment),
                parentId: id,
                manifest,
                tag,
            });
            print(`${name}@${version}`);

            // record each release as it lands, so a run failing midway goes on from it
            const [, release = version, sequence = "1"] = LOCAL_VERSION.exec(version) ?? [];
            packages[name] = { release, sequence: Number(sequence), manifest: digest };
            await this.#record({ packages });
        }

        return new Publication(this.#directory, { packages }, this.#files);
    }

    /** Write the published releases into the state directory. */
    async #record(published: Published): Promise<void> {
        await this.#files.writeTextFile(
            join(this.#directory, FILE),
            `${JSON.stringify(published, null, 4)}\n`,
        );
    }

    /** Connect to a residency home's forge as a machine in the home, once the home serves it. */
    static async #forge(
        machine: Machine,
        home: Identifier<"space">,
        print: (line: string) => void,
    ): Promise<Forge> {
        // wait until the home serves its forge, printing the home's steps and refusing at the bound with the one it waits for
        const { directory, environment } = machine;
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
        const located = await directory.locate(home, environment.clock.now());
        const origin = InstallationMount.url(located.url, home, FORGE_INSTALLATION);
        const url = `${origin}${PackageMount.path(registryPackage.id)}`;

        return {
            service: createClient(registryPackage, { url, fetch }, environment),
            registry: new Registry(async () => ({ url, space: home, fetch }), environment),
        };
    }

    /** Create a package in its account's forge, answering the one it has at the identifier. */
    static async #createPackage(
        forge: ClientOf<typeof registryPackage>,
        spaceId: string,
        accountId: Identifier<"account">,
        described: { readonly id: string; readonly local: string },
        environment: UuidEnvironment,
    ): Promise<void> {
        await forge.package.create({
            spaceId,
            requestId: RequestId.create(environment),
            id: described.id,
            accountId,
            name: described.local,
        });
    }
}
