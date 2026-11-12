import { Directory } from "@destack/account/address";
import { BuildStore, PackageClient } from "@destack/build/store";
import { Claim, ClaimLookup, claimTable } from "@destack/object";
import { type PackageId, PackageName, PackageRelease } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import { type Digest, type Identifier, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { createClient } from "@destack/object/client";
import { Failure } from "@destack/error";
import { RegistryError } from "../error/index.ts";
import { packageObject } from "../object/index.ts";
import registryPackage from "../package.ts";
import type { Environment } from "@destack/environment";
import type { Lease } from "@destack/resource";
import { RequestId } from "@destack/sync";

/** The forge a caller calls for a package. */
type ForgeEndpoint = {
    /** The forge's URL, its package's path. */
    readonly url: string;
    /** The space storing the forge's packages, its residency home. */
    readonly space: string;
    /** Fetch from it as the caller. */
    readonly fetch: Fetch;
};

/** Whose forge a caller calls: a package's publisher's, or an account's. */
type ForgeOwner = { readonly package: PackageId } | { readonly account: Identifier<"account"> };

/** The forges of published releases and repositories, called as one caller. */
export class Registry {
    /** Find the forge of a package's publisher or of an account. */
    readonly #find: (owner: ForgeOwner) => Promise<ForgeEndpoint>;
    /** The environment the forge's clients run in. */
    readonly #environment: Environment;
    /** The releases this caller ships, whose manifests it finds before any forge's. */
    readonly #releases: readonly PackageRelease[];
    /** The reader of each build opened so far, by package and manifest, as builds never change. */
    readonly #readers = new Map<string, Promise<BuildReader>>();

    /** Call the forge of each package's publisher and each account, after the shipped releases. */
    constructor(
        find: (owner: ForgeOwner) => Promise<ForgeEndpoint>,
        environment: Environment,
        releases: readonly PackageRelease[] = [],
    ) {
        this.#find = find;
        this.#environment = environment;
        this.#releases = releases;
    }

    /** Call forges through the universe directory's replica, signing for each forge's home. */
    static of(
        directory: Directory,
        home: (account: Identifier<"account">) => Promise<Identifier<"space">>,
        sign: (audience: PackageId, home: Identifier<"space">) => Fetch,
        environment: Environment,
        installation: string,
        releases: readonly PackageRelease[] = [],
    ): Registry {
        const claims = ClaimLookup.of(directory.snapshot.read(claimTable));

        return new Registry(
            async (owner) => {
                // call the forge below its installation in the home of the owning account's residency
                const accountId = await Registry.#account(claims, owner);
                const spaceId = await home(accountId);
                const now = environment.clock.now();
                const url = await directory.mount(spaceId, installation, registryPackage.id, now);

                return { url, space: spaceId, fetch: sign(registryPackage.id, spaceId) };
            },
            environment,
            releases,
        );
    }

    /** Read the account owning a forge: an account itself, or the one publishing a package. */
    static async #account(claims: ClaimLookup, owner: ForgeOwner): Promise<Identifier<"account">> {
        // take an account itself
        if ("account" in owner) {
            return owner.account;
        }

        // find the account claiming the package's identifier
        const found = await Claim.lookup(packageObject, claims, "id", [owner.package]);
        if (found === undefined) {
            throw RegistryError.of(
                "PUBLISHER_NOT_FOUND",
                `publishing account for '${owner.package}' not found`,
            );
        }

        return schema.identifier("account").parse(found.scope);
    }

    /** Resolve a package's published release of a version, or of the version a tag points at. */
    async resolve(packageId: PackageId, release: string) {
        const forge = await this.#find({ package: packageId });

        return createClient(registryPackage, forge, this.#environment).release.resolve({
            spaceId: forge.space,
            parentId: packageId,
            release,
        });
    }

    /** Lease short-lived access to read a repository at the forge of its account. */
    async lease(
        account: Identifier<"account">,
        repositoryId: Identifier<"repository">,
    ): Promise<Lease> {
        const forge = await this.#find({ account });
        const client = createClient(registryPackage, forge, this.#environment);

        return client.repository.open({ spaceId: forge.space, id: repositoryId, mode: "read" });
    }

    /** Find the digest of the manifest of a shipped release, or of a package's published one. */
    async find(packageId: PackageId, release: string): Promise<Digest> {
        // take a shipped release first, a dev build of it among them
        const shipped = PackageRelease.select(this.#releases, packageId, release);
        if (shipped !== undefined) {
            return shipped.manifest;
        }

        // resolve the release at its forge
        const { manifest } = await this.resolve(packageId, release);

        return manifest;
    }

    /** Push a stored build and its npm units to its package's forge. */
    async push(
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
        signal?: AbortSignal,
    ): Promise<Digest> {
        // find the forge of the build's package
        const build = await store.contents(manifest);
        const packageId = PackageManifest.package(build.manifest).id;
        const forge = await this.#find({ package: packageId });
        await Registry.#push(forge, store, manifest, packageId, signal, this.#environment);

        return manifest;
    }

    /** Release a stored build under a tag at its account's forge, or point the tag at it. */
    async release(
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
        account: Identifier<"account">,
        tag: string,
    ): Promise<"released" | "pointed"> {
        // create the build's package once at the account's forge
        const build = await store.contents(manifest);
        const { id, name, version } = PackageManifest.package(build.manifest);
        const forge = await this.#find({ account });
        const client = createClient(registryPackage, forge, this.#environment);
        const spaceId = forge.space;
        await client.package.create({
            spaceId,
            requestId: RequestId.create(this.#environment),
            id,
            accountId: account,
            name: PackageName.split(name).local,
        });

        // read the release the forge has at the version
        const listed = await client.release.list({
            spaceId,
            where: { parentId: id, version },
            limit: 1,
        });
        const [published] = listed.items;

        // push and release a version the forge lacks, under the tag
        if (published === undefined) {
            await Registry.#push(forge, store, manifest, id, undefined, this.#environment);
            await client.release.create({
                spaceId,
                requestId: RequestId.create(this.#environment),
                parentId: id,
                manifest,
                tag,
            });

            return "released";
        }
        // refuse a version the forge has from another build
        else if (published.manifest !== manifest) {
            throw RegistryError.of(
                "RELEASE_EXISTS",
                `release '${name}@${version}' already exists: another build published it`,
            );
        }
        // point the tag at the version the forge has from this build
        else {
            await client.package.point({
                spaceId,
                id,
                requestId: RequestId.create(this.#environment),
                name: tag,
                version,
            });

            return "pointed";
        }
    }

    /** Publish a stored build under a tag, after the account's unpublished builds it pins. */
    async publish(
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
        account: Identifier<"account">,
        tag: string,
    ): Promise<readonly PackageRelease[]> {
        // layer the build's closure, each layer pinning only builds of earlier ones
        const root = PackageManifest.package((await store.contents(manifest)).manifest);
        const { handle } = PackageName.split(root.name);
        const layers = await BuildStore.layers(store, [manifest]);

        // release a layer at a time
        const released: PackageRelease[] = [];
        for (const layer of layers) {
            const outcomes = await Promise.all(
                layer.map(async (digest): Promise<PackageRelease[]> => {
                    // require another account's build published, else release this account's
                    const built = PackageManifest.package((await store.contents(digest)).manifest);
                    if (PackageName.split(built.name).handle !== handle) {
                        await this.#requirePublished(built, digest);

                        return [];
                    } else {
                        const outcome = await this.release(store, digest, account, tag);

                        return outcome === "released" ? [{ package: built, manifest: digest }] : [];
                    }
                }),
            );
            released.push(...outcomes.flat());
        }

        return released;
    }

    /** Refuse a build of another account that its forge has not released at its version. */
    async #requirePublished(built: PackageRelease["package"], manifest: Digest): Promise<void> {
        // read the published build of its version, none where its forge has no such release
        const published = await this.find(built.id, built.version).catch((error: unknown) => {
            if (Failure.of(error).code !== "NOT_FOUND") {
                throw error;
            }

            return undefined;
        });

        // refuse a version unpublished, or published from another build
        if (published !== manifest) {
            throw RegistryError.of(
                "DEPENDENCY_BLOCKED",
                `cannot publish this build: dependency '${built.name}@${built.version}' is not published`,
            );
        }
    }

    /** Push a stored build and its npm units below its package at a forge. */
    static async #push(
        forge: ForgeEndpoint,
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
        packageId: PackageId,
        signal: AbortSignal | undefined,
        environment: Environment,
    ): Promise<void> {
        // upload below the package, which its forge stages for a publisher to release
        const uploads = `${forge.url}${registryPackage.mounts.builds.path}/${packageId}/`;
        const client = new PackageClient(uploads, forge.fetch, environment);

        // push each npm unit, then the build
        const units = await store.units(manifest, packageId);
        for (const subject of [...units, manifest]) {
            await client.push(await store.contents(subject), signal);
        }
    }

    /** Locate the files of a package's published build by its manifest's digest. */
    async locate(
        packageId: PackageId,
        manifest: Digest,
    ): Promise<{
        readonly location: PackageLocation;
        readonly fetch: (input: URL, init: RequestInit) => Promise<Response>;
    }> {
        const forge = await this.#find({ package: packageId });

        return {
            location: {
                manifest,
                url: `${forge.url}${registryPackage.mounts.builds.path}/${manifest}/`,
            },
            fetch: (input, request) => forge.fetch(new Request(input.href, request)),
        };
    }

    /** Read the build a published build attached at a path, such as its runtime build. */
    async referrer(
        packageId: PackageId,
        manifest: Digest,
        path: string,
    ): Promise<Digest | undefined> {
        // ask the build's forge for the referrer at the path, as OCI's referrers API answers
        const { location, fetch } = await this.locate(packageId, manifest);
        const url = new URL(`referrers/${encodeURIComponent(path)}`, location.url);
        const response = await fetch(url, {});
        if (response.status === 404) {
            await response.body?.cancel();

            return undefined;
        }
        if (!response.ok) {
            await response.body?.cancel();
            throw RegistryError.of(
                "REMOTE_UNREACHABLE",
                `could not read '${path}' of '${manifest}' from the forge: HTTP ${response.status}`,
            );
        }

        return PackageFile.parse(await response.json()).digest;
    }

    /** Open the build of a package's published release by its manifest's digest, file by file. */
    read(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        // open each build once, forgetting a failed open
        const key = `${packageId} ${manifest}`;
        let reader = this.#readers.get(key);
        if (reader === undefined) {
            reader = this.#open(packageId, manifest);
            this.#readers.set(key, reader);
            reader.catch(() => this.#readers.delete(key));
        }

        return reader;
    }

    /** Open the build the forge serves by its digest, refusing another package's build. */
    async #open(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        // read the build the forge serves by its manifest's digest
        const { location, fetch } = await this.locate(packageId, manifest);
        const reader = await BuildReader.open(location, fetch, this.#environment.crypto);

        // require a build of the package
        const built = PackageManifest.package(reader.manifest);
        if (built.id !== packageId) {
            throw RegistryError.of(
                "MANIFEST_INVALID",
                `build '${manifest}' is invalid: it is a build of '${built.name}' instead of the requested package`,
            );
        }

        return reader;
    }

    /** The releases the machine's distribution ships, run by scopes that install none of them. */
    get distribution(): readonly PackageRelease[] {
        return this.#releases;
    }

    /** Open the build of a package's published release, by its version or a distribution tag. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        return this.read(packageId, await this.find(packageId, release));
    }
}
