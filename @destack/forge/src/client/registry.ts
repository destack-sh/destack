import { Directory } from "@destack/account/address";
import { PackageClient } from "@destack/build/store";
import { Claim, ClaimLookup, claimTable } from "@destack/object";
import type { PackageId } from "@destack/package";
import { BuildReader, type PackageLocation } from "@destack/package/manifest";
import { type Identifier, schema } from "@destack/schema";
import { type Fetch, InstallationMount, PackageMount } from "@destack/runtime";
import { createClient } from "@destack/runtime/client";
import { CallError } from "@destack/runtime/error";
import { packageObject } from "../object/index.ts";
import forgePackage, { FORGE_INSTALLATION } from "../package.ts";
import type { Clock } from "@destack/environment";

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

    /** Read releases from the forge each package's publisher stores them in. */
    constructor(find: (packageId: PackageId) => Promise<ForgeEndpoint>) {
        this.#find = find;
    }

    /** Read releases through the universe directory's replica: claims, placements, endpoints. */
    static of(
        directory: Directory,
        home: (account: Identifier<"account">) => Promise<Identifier<"space">>,
        fetch: Fetch,
        clock: Clock,
    ): Registry {
        const claims = ClaimLookup.of(directory.snapshot.read(claimTable));

        return new Registry(async (packageId) => {
            // find the account claiming the package's identifier
            const found = await Claim.lookup(packageObject, claims, "id", [packageId]);
            if (found === undefined) {
                throw new CallError("NOT_FOUND", {
                    message: `no account publishes ${packageId}`,
                });
            }

            // find the machine serving the home of the account's residency
            const accountId = schema.identifier("account").parse(found.scope);
            const spaceId = await home(accountId);
            const located = await directory.locate(spaceId, clock.now());

            // call the forge's package below its installation in the home
            const installation = InstallationMount.url(located.url, spaceId, FORGE_INSTALLATION);

            return {
                url: `${installation}${PackageMount.path(forgePackage.id)}`,
                space: spaceId,
                fetch,
            };
        });
    }

    /** Resolve a package's published release of a version, or of the version a tag points at. */
    async resolve(packageId: PackageId, release: string) {
        const forge = await this.#find(packageId);

        return createClient(forgePackage, forge).release.resolve({
            spaceId: forge.space,
            parentId: packageId,
            release,
        });
    }

    /** Find the digest of the manifest of a package's published release. */
    async find(packageId: PackageId, release: string): Promise<string> {
        const { manifest } = await this.resolve(packageId, release);

        return manifest;
    }

    /** Push a build's files to its package's forge, answering its manifest's digest. */
    async push(build: BuildReader, signal?: AbortSignal): Promise<string> {
        // upload below the package, which its forge authorizes to the package's publishers
        const packageId = build.manifest.package.id;
        const forge = await this.#find(packageId);
        const uploads = `${forge.url}${forgePackage.adapters.builds.path}/${packageId}/`;

        return new PackageClient(uploads, forge.fetch).push(build, signal);
    }

    /** Locate the files of a package's published build by its manifest's digest. */
    async locate(
        packageId: PackageId,
        manifest: string,
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

    /** Open the build of a package's published release by its manifest's digest, file by file. */
    async read(packageId: PackageId, manifest: string): Promise<BuildReader> {
        // read the build the forge serves by its manifest's digest
        const { location, fetch } = await this.locate(packageId, manifest);
        const reader = await BuildReader.open(location, fetch);

        // require a build of the package
        if (reader.manifest.package.id !== packageId) {
            throw new CallError("BAD_REQUEST", {
                message: `build ${manifest} is no build of ${packageId}`,
            });
        }

        return reader;
    }

    /** Open the build of a package's published release, by its version or a distribution tag. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        return this.read(packageId, await this.find(packageId, release));
    }
}
