import { Directory } from "@destack/account/address";
import { PackageClient, type PackageStore } from "@destack/build/store";
import { Claim, ClaimLookup, claimTable } from "@destack/object";
import type { PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import { type Digest, type Identifier, schema } from "@destack/schema";
import { type Fetch, InstallationMount, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import { ForgeError } from "../error/index.ts";
import { packageObject } from "../object/index.ts";
import forgePackage, { FORGE_INSTALLATION } from "../package.ts";
import type { Environment } from "@destack/environment";

/** The forge a caller calls for a package. */
type ForgeEndpoint = {
    /** The forge's URL, its package's mount. */
    readonly url: string;
    /** The space storing the forge's packages, its residency home. */
    readonly space: string;
    /** Fetch from it as the caller. */
    readonly fetch: Fetch;
};

/** The registry of published releases. */
export class Registry {
    /** Find the forge of the account publishing a package. */
    readonly #find: (packageId: PackageId) => Promise<ForgeEndpoint>;
    /** The environment the forge's clients run in. */
    readonly #environment: Environment;

    /** Read releases from the forge each package's publisher stores them in, in an environment. */
    constructor(find: (packageId: PackageId) => Promise<ForgeEndpoint>, environment: Environment) {
        this.#find = find;
        this.#environment = environment;
    }

    /** Read releases through the universe directory's replica: claims, placements, endpoints. */
    static of(
        directory: Directory,
        home: (account: Identifier<"account">) => Promise<Identifier<"space">>,
        fetch: Fetch,
        environment: Environment,
    ): Registry {
        const claims = ClaimLookup.of(directory.snapshot.read(claimTable));

        return new Registry(async (packageId) => {
            // find the account claiming the package's identifier
            const found = await Claim.lookup(packageObject, claims, "id", [packageId]);
            if (found === undefined) {
                throw ForgeError.of("PUBLISHER_MISSING", `${packageId} has no publishing account`);
            }

            // find the machine serving the home of the account's residency
            const accountId = schema.identifier("account").parse(found.scope);
            const spaceId = await home(accountId);
            const located = await directory.locate(spaceId, environment.clock.now());

            // call the forge's package below its installation in the home
            const installation = InstallationMount.url(located.url, spaceId, FORGE_INSTALLATION);

            return {
                url: `${installation}${PackageMount.path(forgePackage.id)}`,
                space: spaceId,
                fetch,
            };
        }, environment);
    }

    /** Resolve a package's published release of a version, or of the version a tag points at. */
    async resolve(packageId: PackageId, release: string) {
        const forge = await this.#find(packageId);

        return createClient(forgePackage, forge, this.#environment).release.resolve({
            spaceId: forge.space,
            parentId: packageId,
            release,
        });
    }

    /** Find the digest of the manifest of a package's published release. */
    async find(packageId: PackageId, release: string): Promise<Digest> {
        const { manifest } = await this.resolve(packageId, release);

        return manifest;
    }

    /** Push a stored build and its npm units to its package's forge. */
    async push(store: PackageStore, manifest: Digest, signal?: AbortSignal): Promise<Digest> {
        // upload below the package, which its forge authorizes to the package's publishers
        const build = await store.contents(manifest);
        const packageId = PackageManifest.package(build.manifest).id;
        const forge = await this.#find(packageId);
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
        const forge = await this.#find(packageId);

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
    async read(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
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
