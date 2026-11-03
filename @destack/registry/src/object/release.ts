import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { graph } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import { packageObject, TagName } from "./package.ts";

/** The longest deprecation message: 1 KiB, a paragraph. */
const DEPRECATION_LENGTH = 1024;

/** The warning installers show for a deprecated release. */
const DeprecationMessage = schema.string().min(1).max(DEPRECATION_LENGTH);

/** A pushed build checked for its release, before the release's transaction. */
export const Candidate = schema.object({
    /** The digest of the build's manifest. */
    manifest: Digest,
    /** The commit the build compiled, absent for a prerelease of uncommitted changes. */
    commit: schema.string().exactOptional(),
    /** The version the build declares. */
    version: schema.string(),
    /** The manifests of the npm units the build depends on, directly or through other units. */
    units: schema.array(Digest).readonly(),
    /** The build's declarations, with their terms. */
    declarations: schema.array(graph.Declaration).readonly(),
});
/** A pushed build checked for its release, before the release's transaction. */
export type Candidate = schema.Infer<typeof Candidate>;

/** An immutable release of a package at one calendar version. */
export const release = defineObject(
    {
        name: "release",
        plural: "releases",
        scope: space,
        nested: { in: packageObject, receive: "publish", delete: "restrict" },
        fields: {
            /** The immutable calendar version. */
            version: field.string(),
            /** The SHA-256 digest of the published manifest. */
            manifest: field.blob(),
            /** The full Git SHA-1 or SHA-256 commit identifier, absent for some prereleases. */
            commit: field.string().optional(),
            /** The warning installers show for the release, absent while it is not deprecated. */
            deprecation: field.string(DeprecationMessage).optional(),
            /** When the release was yanked: new installs skip it, exact pins still resolve it. */
            yankedAt: field.time().optional(),
        },
        permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
        constraints: {
            // record the commit of every stable release
            commit: {
                kind: "check",
                condition: {
                    OR: [
                        { commit: { length: { in: [40, 64] }, characters: "0-9a-f" } },
                        { commit: { isNull: true }, NOT: { version: { characters: "0-9." } } },
                    ],
                },
            },
            manifest: {
                kind: "check",
                condition: { manifest: { length: 64, characters: "0-9a-f" } },
            },
            version: { kind: "unique", on: ["parentId", "version"] },
            build: { kind: "index", on: ["manifest"] },
        },
    },
    (method) => ({
        create: method.create("publish", {
            isPredicted: false,
            fields: [],
            prepared: Candidate,
            input: schema.object({
                /** The digest of the pushed build's manifest. */
                manifest: Digest,
                /** The distribution tag pointing at the release, latest when absent. */
                tag: TagName.exactOptional(),
            }),
        }),
        /** Find a package's published release of a version, or of the version a tag points at. */
        resolve: method.query({
            permission: "read",
            target: "scope",
            input: schema.object({
                /** The package. */
                parentId: PackageId,
                /** The version, or a distribution tag pointing at it, such as latest. */
                release: schema.string().min(1),
            }),
        }),
        deprecate: method.mutation({
            permission: "publish",
            input: schema.object({
                /** The warning installers show, or null to withdraw the deprecation. */
                message: DeprecationMessage.nullable(),
            }),
        }),
        yank: method.mutation({
            permission: "publish",
            input: schema.object({
                /** Whether new installs skip the release, false to restore it. */
                isYanked: schema.boolean(),
            }),
        }),
    }),
);

/** A persisted release record. */
export type Release = Select<typeof release.table>;
