import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { Candidate, packageObject, TagName } from "./package.ts";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The longest deprecation message: 1 KiB, a paragraph. */
const DEPRECATION_LENGTH = 1024;

/** The warning installers show for a deprecated release. */
const DeprecationMessage = schema.string().min(1).max(DEPRECATION_LENGTH);

/** An immutable release of a package at one calendar version. */
export const release = defineObject(
    {
        name: "release",
        plural: "releases",
        title: t`${plural(argument("count"), { one: "Release", other: "Releases" })}`,
        scope: space,
        fields: {
            /** The immutable calendar version. */
            version: field.string(),
            /** The published lock's digest: the package's build and every build it runs with. */
            lock: field.blob(),
            /** The full Git SHA-1 or SHA-256 commit identifier, absent for some prereleases. */
            commit: field.string().optional(),
            /** The warning installers show for the release, absent while it is not deprecated. */
            deprecation: field.string(DeprecationMessage).optional(),
            /** When the release was yanked: new installs skip it, exact pins still resolve it. */
            yankedAt: field.time().optional(),
        },
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
            lock: {
                kind: "check",
                condition: { lock: { length: 64, characters: "0-9a-f" } },
            },
            version: { kind: "unique", on: ["parentId", "version"] },
            build: { kind: "index", on: ["lock"] },
        },
        access: {
            permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
        },
        traits: [nested({ in: packageObject, add: "publish", delete: "restrict" })],
    },
    (method) => ({
        /** Insert a checked lock's release under a tag, which only the package's publish calls. */
        create: method.create("publish", {
            visibility: "package",
            isPredicted: false,
            fields: [],
            input: Candidate.extend({
                /** The distribution tag pointing at the release, latest when absent. */
                tag: TagName.exactOptional(),
            }),
        }),
        /** Find a package's published release of a version, or of the version a tag points at. */
        resolve: method.query({
            title: t`Resolve`,
            permission: "read",
            target: "scope",
            input: schema.object({
                /** The package. */
                parentId: PackageId,
                /** The version, or a distribution tag pointing at it, such as latest. */
                release: schema.string().min(1),
            }),
        }),
        /** Delete a prerelease, refusing a stable release, which stays immutable and only yanks. */
        delete: method.delete("publish"),
        deprecate: method.mutation({
            title: t`Deprecate`,
            permission: "publish",
            input: schema.object({
                /** The warning installers show, or null to withdraw the deprecation. */
                message: DeprecationMessage.nullable(),
            }),
        }),
        yank: method.mutation({
            title: t`Yank`,
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
