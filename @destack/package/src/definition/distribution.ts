import { schema } from "@destack/schema";
import { PackageRole } from "./definition.ts";
import { type PackageId, PackageRelease } from "./package.ts";

/** The file of a distribution's directory listing the releases it ships. */
export const DISTRIBUTION_MANIFEST = "distribution.json";

/** The directory of a distribution with its releases' builds, read in place as a layer. */
export const DISTRIBUTION_LAYER = "releases";

/** The module a Worker's bundle reads its distribution's manifest from, supplied by its deploy. */
export const DISTRIBUTION_MODULE = "destack:distribution";

/** The structure of a distribution's manifest. */
const manifest = schema.object({
    /** The shipped releases, each with what its package is. */
    releases: schema.array(PackageRelease.extend({ role: PackageRole })),
});

/** The releases a Destack distribution ships, each with its role, wherever their builds lie. */
export const DistributionManifest = Object.assign(manifest, {
    /** Read the shipped releases without their roles, which machines resolve installations to. */
    shipped(value: DistributionManifest): PackageRelease[] {
        return value.releases.map(({ package: released, manifest: digest }) => ({
            package: released,
            manifest: digest,
        }));
    },

    /** Read the shipped builtins by package, which machines link to the servers they load. */
    builtins(value: DistributionManifest): Map<PackageId, PackageRelease> {
        return new Map(
            value.releases
                .filter((release) => release.role === "builtin")
                .map(({ package: released, manifest: digest }) => [
                    released.id,
                    { package: released, manifest: digest },
                ]),
        );
    },
});
/** The releases a Destack distribution ships. */
export type DistributionManifest = schema.Infer<typeof manifest>;
