import { Account } from "@destack/account/object";
import type { Environment } from "@destack/environment";
import { Registry } from "@destack/registry/client";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import registryPackage from "@destack/registry/package";
import { type PackageId, PackageRelease } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { Digest } from "@destack/schema";
import relayPackage, { TUNNEL_TOKEN_LIFETIME_MILLISECONDS } from "@destack/relay/package";
import type { UniverseOptions } from "@destack/universe";

/** The releases a deploy shipped beside the universe, which it opens before any forge's. */
export interface ShippedReleases {
    /** The shipped releases. */
    readonly releases: readonly PackageRelease[];
    /** Open a shipped release's build by its manifest. */
    readonly open: (manifest: Digest) => Promise<BuildReader>;
}

/** The platform's packages a universe serves beside its own: the forges' and the relay's. */
export const PlatformUniverse = {
    /** Build a universe's options for the forges, the relay and the shipped releases. */
    options(
        environment: Environment,
        shipped: ShippedReleases,
    ): Pick<UniverseOptions, "releases" | "policies" | "lifetimes"> {
        return {
            // open a shipped release first, as forges publish none before the platform runs, else the forges' through the directory
            releases: ({ directory, connect }) => {
                const forges = Registry.of(
                    directory,
                    (account) => Account.locate(directory.snapshot, account),
                    connect,
                    environment,
                    FORGE_INSTALLATION,
                );

                return {
                    open: async (packageId, release) => {
                        const found = PackageRelease.select(shipped.releases, packageId, release);

                        return found === undefined
                            ? forges.open(packageId, release)
                            : shipped.open(found.manifest);
                    },
                };
            },
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
