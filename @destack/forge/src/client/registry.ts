import { Directory } from "@destack/account/address";
import { PackageClient, type PackageStore } from "@destack/build/store";
import { Claim, ClaimLookup, claimTable } from "@destack/object";
import type { PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import { type Digest, type Identifier, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { createClient } from "@destack/object/client";
import { ForgeError } from "../error/index.ts";
import { packageObject } from "../object/index.ts";
import forgePackage, { FORGE_INSTALLATION } from "../package.ts";
import type { Environment } from "@destack/environment";
import type { Lease } from "@destack/resource";

/** The forge a caller calls for a package. */
type ForgeEndpoint = {
    /** The forge's URL, its package's mount. */
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
    /** The reader of each build opened so far, by package and manifest, as builds never change. */
    readonly #readers = new Map<string, Promise<BuildReader>>();

    /** Call the forge of each package's publisher and each account, in an environment. */
    constructor(find: (owner: ForgeOwner) => Promise<ForgeEndpoint>, environment: Environment) {
        this.#find = find;
        this.#environment = environment;
    }

    /** Call forges through the universe directory's replica, signing for the forge package. */
    static of(
        directory: Directory,
        home: (account: Identifier<"account">) => Promise<Identifier<"space">>,
        sign: (audience: PackageId) => Fetch,
        environment: Environment,
    ): Registry {
        const claims = ClaimLookup.of(directory.snapshot.read(claimTable));

        return new Registry(async (owner) => {
            // call the forge below its installation in the home of the owning account's residency
            const accountId = await Registry.#account(claims, owner);
            const spaceId = await home(accountId);
            const now = environment.clock.now();
            const url = await directory.mount(spaceId, FORGE_INSTALLATION, forgePackage.id, now);

            return { url, space: spaceId, fetch: sign(forgePackage.id) };
        }, environment);
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
            throw ForgeError.of("PUBLISHER_MISSING", `${owner.package} has no publishing account`);
        }

        return schema.identifier("account").parse(found.scope);
    }

    /** Resolve a package's published release of a version, or of the version a tag points at. */
    async resolve(packageId: PackageId, release: string) {
        const forge = await this.#find({ package: packageId });

        return createClient(forgePackage, forge, this.#environment).release.resolve({
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
        const client = createClient(forgePackage, forge, this.#environment);

        return client.repository.open({ spaceId: forge.space, id: repositoryId, mode: "read" });
    }

    /** Find the digest of the manifest of a package's published release. */
    async find(packageId: PackageId, release: string): Promise<Digest> {
        const { manifest } = await this.resolve(packageId, release);

        return manifest;
    }

    /** Push a stored build and its npm units to its package's forge. */
    async push(
        store: Pick<PackageStore, "contents" | "units">,
        manifest: Digest,
        signal?: AbortSignal,
    ): Promise<Digest> {
        // upload below the package, which its forge authorizes to the package's publishers
        const build = await store.contents(manifest);
        const packageId = PackageManifest.package(build.manifest).id;
        const forge = await this.#find({ package: packageId });
        const uploads = `${forge.url}${forgePackage.adapters.builds.path}/${packageId}/`;
        const client = new PackageClient(uploads, forge.fetch, this.#environment.crypto);

        // push each npm unit, then the build
        const units = await store.units(manifest, packageId);
        for (const subject of [...units, manifest]) {
            await client.push(await store.contents(subject), signal);
        }

        return manifest;
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
                url: `${forge.url}${forgePackage.adapters.builds.path}/${manifest}/`,
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
            throw ForgeError.of(
                "REMOTE_FAILED",
                `the forge answered ${path} of ${manifest} with HTTP ${response.status}`,
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
        if (PackageManifest.package(reader.manifest).id !== packageId) {
            throw ForgeError.of(
                "FOREIGN_MANIFEST",
                `build ${manifest} is not a build of ${packageId}`,
            );
        }

        return reader;
    }

    /** Open the build of a package's published release, by its version or a distribution tag. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        return this.read(packageId, await this.find(packageId, release));
    }
}
