import type { Host } from "@destack/environment";
import { Digest, found, present, schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";
import { PackageRole } from "./definition.ts";
import { type PackageId, type PackageRelease, PackageVersion } from "./package.ts";

/** The file of a distribution's directory listing the releases it ships. */
export const DISTRIBUTION_MANIFEST = "distribution.json";

/** The directory of a distribution with its releases' builds, read in place as a layer. */
export const DISTRIBUTION_LAYER = "releases";

/** The module a Worker's bundle reads its distribution's manifest from, supplied by its deploy. */
export const DISTRIBUTION_MODULE = "destack:distribution";

/** The platforms an image runs on: a Bun host's `os-architecture`, or workerd. */
const platformSchema = schema.enum([
    "darwin-arm64",
    "darwin-x64",
    "linux-arm64",
    "linux-x64",
    "win32-arm64",
    "win32-x64",
    "workerd",
]);

/** The platform a distribution's image runs on, as its manifest keys the image. */
export const DistributionPlatform = Object.assign(platformSchema, {
    /** Read the platform of a Bun host. */
    of(host: Pick<Host, "os" | "architecture">): DistributionPlatform {
        return `${host.os}-${host.architecture}`;
    },
});
/** The platform a distribution's image runs on. */
export type DistributionPlatform = schema.Infer<typeof platformSchema>;

/** The structure of a distribution's manifest. */
const manifest = schema.object({
    /** The shipped releases, each with what its package is and its image on each platform. */
    releases: schema.array(
        schema.object({
            /** The released package name and version. */
            package: PackageVersion,
            /** What the package is. */
            role: PackageRole,
            /** The digest of the release's image on each platform it ships for. */
            images: schema.partialRecord(platformSchema, Digest),
        }),
    ),
    /** Whether the distribution ships its releases' builds beside the manifest as a layer. */
    isLayered: schema.boolean(),
});

/** The releases a Destack distribution ships, each with its role, wherever their builds lie. */
export const DistributionManifest = Object.assign(manifest, {
    /** Read the releases with their images on a platform, as machines resolve them. */
    releases(value: DistributionManifest, platform: DistributionPlatform): PackageRelease[] {
        return value.releases.map((release) => ({
            package: release.package,
            image: present(
                release.images[platform],
                `the ${platform} image of ${release.package.name}`,
            ),
        }));
    },

    /** Read the shipped builtins with their images on a platform, by package. */
    builtins(
        value: DistributionManifest,
        platform: DistributionPlatform,
    ): Map<PackageId, PackageRelease> {
        const builtins = value.releases.filter((release) => release.role === "builtin");

        return new Map(
            DistributionManifest.releases({ ...value, releases: builtins }, platform).map(
                (release) => [release.package.id, release],
            ),
        );
    },

    /** Merge the manifests of one distribution built on several platforms. */
    merge(left: DistributionManifest, right: DistributionManifest): DistributionManifest {
        // refuse a layer only one platform ships
        if (left.isLayered !== right.isLayered) {
            throw PackageError.of(
                "DISTRIBUTION_INVALID",
                "distribution is invalid: only one platform ships a layer",
            );
        }

        // refuse a release only one manifest ships
        const lefts = new Set(left.releases.map((release) => release.package.id));
        const rights = new Map(right.releases.map((release) => [release.package.id, release]));
        const [unpaired] = [
            ...left.releases.filter((release) => !rights.has(release.package.id)),
            ...right.releases.filter((release) => !lefts.has(release.package.id)),
        ];
        if (unpaired !== undefined) {
            throw PackageError.of(
                "DISTRIBUTION_INVALID",
                `distribution is invalid: only one platform ships '${describe(unpaired)}'`,
            );
        }

        // union each release's images, refusing a different release or two images on one platform
        const releases = left.releases.map((release) => {
            // find the release's counterpart and the platform both image differently
            const other = found(rights, release.package.id);
            const [shown, otherShown] = [describe(release), describe(other)];
            const conflict = platformSchema.options.find(
                (each) =>
                    release.images[each] !== undefined &&
                    other.images[each] !== undefined &&
                    release.images[each] !== other.images[each],
            );
            if (shown !== otherShown) {
                throw PackageError.of(
                    "DISTRIBUTION_INVALID",
                    `distribution is invalid: its platforms ship '${shown}' and '${otherShown}'`,
                );
            }
            // refuse two images of one release on a platform
            else if (conflict !== undefined) {
                throw PackageError.of(
                    "DISTRIBUTION_INVALID",
                    `distribution is invalid: it has two '${conflict}' images of '${shown}'`,
                );
            }

            return { ...release, images: { ...release.images, ...other.images } };
        });

        return { releases, isLayered: left.isLayered };
    },
});
/** The releases a Destack distribution ships. */
export type DistributionManifest = schema.Infer<typeof manifest>;

/** Describe a shipped release by its package, version and role. */
function describe(release: DistributionManifest["releases"][number]): string {
    return `${release.package.name}@${release.package.version} (${release.role})`;
}
