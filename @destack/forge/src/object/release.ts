import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import { Distribution, Pack, PackageMetadata } from "../pack/index.ts";
import { packageObject, TagName } from "./package.ts";

/** The longest deprecation message: 1 KiB, a paragraph, repeated per version in packuments. */
const DEPRECATION_LENGTH = 1024;

/** The warning installers show for a deprecated release. */
const DeprecationMessage = schema.string().min(1).max(DEPRECATION_LENGTH);

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
            /** The npm archive installers download. */
            distribution: field.json(Distribution),
            /** The npm declaration installers read. */
            metadata: field.json(PackageMetadata),
            /** The warning installers show for the release, absent while it is not deprecated. */
            deprecation: field.string(DeprecationMessage).optional(),
            /** When the release was yanked: new installs skip it, exact pins still resolve it. */
            yankedAt: field.time().optional(),
        },
        permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
        constraints: {
            // record the commit of every stable release, as npm's gitHead
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
            prepared: Pack,
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
