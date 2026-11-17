import { schema, Version, type JsonObject } from "@destack/schema";

/** The fields of a release's package file an npm client reads before it downloads the release. */
const INSTALL_FIELDS = ["dependencies", "peerDependencies", "optionalDependencies", "bin"] as const;

/** One release as an npm client lists it: its package file, its tarball and the tarball's hash. */
export interface NpmRelease {
    /** The release's package file, at released versions. */
    readonly manifest: JsonObject;
    /** The address of the release's tarball. */
    readonly tarball: string;
    /** The tarball's subresource integrity: `sha512-` and the base64 hash. */
    readonly integrity: string;
    /** The warning installers show for the release, absent while it is not deprecated. */
    readonly deprecation?: string;
}

/** A package as an npm client lists it: its tags and each release it can install. */
export const NpmPackument = {
    /** Describe a package's releases and tags in npm's abbreviated form, newest release last. */
    of(
        name: string,
        tags: Readonly<Record<string, string>>,
        releases: readonly NpmRelease[],
    ): JsonObject {
        // list each release under its version, in release order
        const listed = releases
            .map((release) => ({
                release,
                version: schema.string().parse(release.manifest["version"]),
            }))
            .toSorted((left, right) => Version.compare(left.version, right.version));
        const versions = Object.fromEntries(
            listed.map(({ release, version }) => [
                version,
                {
                    name,
                    version,
                    ...Object.fromEntries(
                        INSTALL_FIELDS.flatMap((field) => {
                            const value = release.manifest[field];

                            return value === undefined ? [] : [[field, value]];
                        }),
                    ),
                    dist: { tarball: release.tarball, integrity: release.integrity },
                    ...(release.deprecation === undefined
                        ? {}
                        : { deprecated: release.deprecation }),
                },
            ]),
        );

        // point each tag at its version, leaving a tag whose release is not listed
        const pointed = Object.fromEntries(
            Object.entries(tags)
                .filter(([, version]) => Object.hasOwn(versions, version))
                .toSorted(([left], [right]) => (left < right ? -1 : 1)),
        );

        return { name, "dist-tags": pointed, versions };
    },
};
