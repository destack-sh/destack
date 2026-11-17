import { schema, Version, type JsonObject } from "@destack/schema";

/** The fields of a release's package file an npm client reads before it downloads the release. */
const INSTALL_FIELDS = ["dependencies", "peerDependencies", "optionalDependencies", "bin"] as const;

/** What an npm client lists of a release, kept with it: its install fields and tarball hash. */
export const NpmRelease = Object.assign(
    schema.object({
        /** The fields of the released package file an npm client reads before it downloads. */
        manifest: schema.jsonObject(),
        /** The tarball's subresource integrity: `sha512-` and the base64 hash. */
        integrity: schema.string().min(1),
    }),
    {
        /** Read the fields of a released package file an npm client reads before it downloads. */
        installed(manifest: JsonObject): JsonObject {
            return Object.fromEntries(
                INSTALL_FIELDS.flatMap((field) => {
                    const value = manifest[field];

                    return value === undefined ? [] : [[field, value]];
                }),
            );
        },
    },
);
/** What an npm client lists of a release. */
export type NpmRelease = schema.Infer<typeof NpmRelease>;

/** One release in a package's listing: its version, what npm lists of it, and its tarball. */
export interface NpmVersion {
    /** The release's version. */
    readonly version: string;
    /** What an npm client lists of the release. */
    readonly listed: NpmRelease;
    /** The address of the release's tarball. */
    readonly tarball: string;
    /** The warning installers show for the release, absent while it is not deprecated. */
    readonly deprecation?: string;
}

/** A package as an npm client lists it: its tags and each release it can install. */
export const NpmPackument = {
    /** Describe a package's releases and tags in npm's abbreviated form, newest release last. */
    of(
        name: string,
        tags: Readonly<Record<string, string>>,
        releases: readonly NpmVersion[],
    ): JsonObject {
        // list each release under its version, in release order
        const listed = releases.toSorted((left, right) =>
            Version.compare(left.version, right.version),
        );
        const versions = Object.fromEntries(
            listed.map((release) => [
                release.version,
                {
                    name,
                    version: release.version,
                    ...release.listed.manifest,
                    dist: { tarball: release.tarball, integrity: release.listed.integrity },
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
