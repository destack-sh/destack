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

/** The platforms a lock runs on: a Bun host's `os-architecture`, or workerd. */
const platformSchema = schema.enum([
    "darwin-arm64",
    "darwin-x64",
    "linux-arm64",
    "linux-x64",
    "win32-arm64",
    "win32-x64",
    "workerd",
]);

/** The platform a distribution's lock runs on, as its manifest keys the lock. */
export const DistributionPlatform = Object.assign(platformSchema, {
    /** Read the platform of a Bun host. */
    of(host: Pick<Host, "os" | "architecture">): DistributionPlatform {
        return `${host.os}-${host.architecture}`;
    },
});
/** The platform a distribution's lock runs on. */
export type DistributionPlatform = schema.Infer<typeof platformSchema>;

/** The structure of a distribution's manifest. */
const manifest = schema.object({
    /** The shipped releases, each with what its package is and its lock on each platform. */
    releases: schema.array(
        schema.object({
            /** The released package name and version. */
            package: PackageVersion,
            /** What the package is. */
            role: PackageRole,
            /** The digest of the release's lock on each platform it ships for. */
            locks: schema.partialRecord(platformSchema, Digest),
        }),
    ),
});

/** The releases a Destack distribution ships, each with its role, wherever their builds lie. */
export const DistributionManifest = Object.assign(manifest, {
    /** Read the releases with their locks on a platform, as machines resolve them. */
    releases(value: DistributionManifest, platform: DistributionPlatform): PackageRelease[] {
        return value.releases.map((release) => ({
            package: release.package,
            lock: present(
                release.locks[platform],
                `the ${platform} lock of ${release.package.name}`,
            ),
        }));
    },

    /** Read the shipped builtins with their locks on a platform, by package. */
    builtins(
        value: DistributionManifest,
        platform: DistributionPlatform,
    ): Map<PackageId, PackageRelease> {
        const builtins = value.releases.filter((release) => release.role === "builtin");

        return new Map(
            DistributionManifest.releases({ releases: builtins }, platform).map((release) => [
                release.package.id,
                release,
            ]),
        );
    },

    /** Merge the manifests of one distribution built on several platforms. */
    merge(left: DistributionManifest, right: DistributionManifest): DistributionManifest {
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

        // union each release's locks, refusing a different release or two locks on one platform
        const releases = left.releases.map((release) => {
            // find the release's counterpart and the platform both lock differently
            const other = found(rights, release.package.id);
            const [shown, otherShown] = [describe(release), describe(other)];
            const conflict = platformSchema.options.find(
                (each) =>
                    release.locks[each] !== undefined &&
                    other.locks[each] !== undefined &&
                    release.locks[each] !== other.locks[each],
            );
            if (shown !== otherShown) {
                throw PackageError.of(
                    "DISTRIBUTION_INVALID",
                    `distribution is invalid: its platforms ship '${shown}' and '${otherShown}'`,
                );
            }
            // refuse two locks of one release on a platform
            else if (conflict !== undefined) {
                throw PackageError.of(
                    "DISTRIBUTION_INVALID",
                    `distribution is invalid: it has two '${conflict}' locks of '${shown}'`,
                );
            }

            return { ...release, locks: { ...release.locks, ...other.locks } };
        });

        return { releases };
    },
});
/** The releases a Destack distribution ships. */
export type DistributionManifest = schema.Infer<typeof manifest>;

/** Describe a shipped release by its package, version and role. */
function describe(release: DistributionManifest["releases"][number]): string {
    return `${release.package.name}@${release.package.version} (${release.role})`;
}
