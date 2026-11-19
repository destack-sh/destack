import { Account } from "@destack/account/object";
import { RemoteBuildStore } from "@destack/build/store";
import { packageObject } from "@destack/registry/object";
import { RegistryError } from "@destack/registry/error";
import type { Environment } from "@destack/environment";
import forgePackage from "@destack/forge/package";
import registryPackage from "@destack/registry/package";
import { type PackageId, PackageRelease } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { Digest } from "@destack/schema";
import relayPackage, { TUNNEL_TOKEN_LIFETIME_MILLISECONDS } from "@destack/relay/package";
import type { ControlOptions } from "@destack/control";

/** The releases a deploy shipped beside the universe, which it opens before any forge's. */
export interface ShippedReleases {
    /** The shipped releases. */
    readonly releases: readonly PackageRelease[];
    /** Open a shipped release's root build by its image. */
    readonly open: (image: Digest) => Promise<BuildReader>;
}

/** The platform's packages a universe serves beside its own: the forges' and the relay's. */
export const PlatformUniverse = {
    /** Build a universe's options for the forges, the relay and the shipped releases. */
    options(
        environment: Environment,
        shipped: ShippedReleases,
    ): Pick<ControlOptions, "releases" | "policies" | "lifetimes"> {
        return {
            // open a shipped release first, as forges publish none before the platform runs, else the forges' through the directory
            releases: ({ directory, connect }) => ({
                open: async (packageId, named) => {
                    // open a shipped release first
                    const shippedRelease = PackageRelease.select(
                        shipped.releases,
                        packageId,
                        named,
                    );
                    if (shippedRelease !== undefined) {
                        return await shipped.open(shippedRelease.image);
                    }

                    // find the forge in the publisher's residency home
                    const owner = await directory.owner(packageObject, packageId);
                    if (owner === undefined) {
                        throw RegistryError.of(
                            "PUBLISHER_NOT_FOUND",
                            `owning account for '${packageId}' not found`,
                        );
                    }
                    const home = await Account.locate(directory.snapshot, owner);
                    const fetch = connect(registryPackage.id);

                    // resolve the release there, then open its root build
                    const registry = directory.client(registryPackage, home, fetch, environment);
                    const found = await registry.release.resolve({
                        spaceId: home,
                        parentId: packageId,
                        release: named,
                    });
                    const mount = await directory.package(
                        home,
                        registryPackage.id,
                        environment.clock.now(),
                    );
                    const builds = `${mount}${registryPackage.mounts.builds.path}`;

                    return await RemoteBuildStore.open(
                        builds,
                        fetch,
                        found.image,
                        environment.crypto,
                    );
                },
                distribution: shipped.releases,
            }),
            policies: [
                ...Object.values(forgePackage.objects),
                ...Object.values(registryPackage.objects),
            ].map((object) => object.policy),
            lifetimes: new Map<PackageId, number>([
                [relayPackage.id, TUNNEL_TOKEN_LIFETIME_MILLISECONDS],
            ]),
        };
    },
};
