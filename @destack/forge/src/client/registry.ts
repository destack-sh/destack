import type { DirectoryClient } from "@destack/account/client";
import type { PackageId } from "@destack/package";
import { BuildReader, type PackageLocation } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import { ServiceError } from "@destack/service/error";
import { packageObject } from "../object/index.ts";
import { BUILDS_PATH, forgeService } from "../service/index.ts";
import { connect } from "./client.ts";

/** The forge a caller reaches for a package: its service's URL and the fetch authenticating the caller. */
type Endpoint = {
    /** The forge service's URL. */
    readonly url: string;
    /** Fetch from it as the caller. */
    readonly fetch: (request: Request) => Promise<Response>;
};

/** The registry of published releases, each package's read from the forge of the account publishing it. */
export class Registry {
    /** Reach the forge of the account publishing a package. */
    readonly #reach: (packageId: PackageId) => Promise<Endpoint>;

    /** Read releases from the forge each package's publisher keeps them in. */
    constructor(reach: (packageId: PackageId) => Promise<Endpoint>) {
        this.#reach = reach;
    }

    /** Read releases through the directory: the account claiming a package's identifier, and the forge of its residency. */
    static of(directory: DirectoryClient, fetch: Endpoint["fetch"]): Registry {
        return new Registry(Registry.reach(directory, fetch));
    }

    /** Reach the forge of the account publishing a package through the directory, as a caller fetching. */
    static reach(
        directory: DirectoryClient,
        fetch: Endpoint["fetch"],
    ): (packageId: PackageId) => Promise<Endpoint> {
        return async (packageId) => {
            // find the account claiming the package's identifier
            const found = await packageObject.lookup(directory, "id", [packageId]);
            if (found === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `no account publishes ${packageId}`,
                });
            }

            // find the region running the forge for the account's residency
            const accountId = schema.identifier("account").parse(found.scope);
            const cell = await directory.region(accountId, forgeService.package.id);
            if (cell === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `no region runs the forge for ${accountId}`,
                });
            }

            return { url: ServiceMount.url(cell.endpoint, forgeService.package.id), fetch };
        };
    }

    /** Find the digest of the manifest of a package's published release, by its version or a distribution tag. */
    async find(packageId: PackageId, release: string): Promise<string> {
        const { manifest } = await connect(await this.#reach(packageId)).manifests.find({
            packageId,
            release,
        });

        return manifest;
    }

    /** Locate the files of a package's published build by its manifest's digest, with the fetch reading them. */
    async locate(
        packageId: PackageId,
        manifest: string,
    ): Promise<{
        readonly location: PackageLocation;
        readonly fetch: (input: URL, init: RequestInit) => Promise<Response>;
    }> {
        const forge = await this.#reach(packageId);

        return {
            location: { manifest, url: `${forge.url}${BUILDS_PATH}${manifest}/` },
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
            throw new ServiceError("BAD_REQUEST", {
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
