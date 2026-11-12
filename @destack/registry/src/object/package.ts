import { resource, through, union, universe } from "@destack/access";
import { account, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Digest, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { graph, PackageId, PackageName } from "@destack/package";
import { Vocabulary } from "@destack/resource";
import { RegistryError } from "../error/index.ts";
import { validRange } from "semver";
import { argument, plural, t } from "@destack/locale";

/** The longest package name in an account: npm's 214 minus a 63 character handle, `@` and `/`. */
const NAME_LENGTH = 149;

/** The longest distribution tag name, as long as the longest version it may stand for. */
const TAG_LENGTH = 256;

/** Who can read a package: anyone, anyone with its name, or its account alone. */
const VISIBILITIES = ["public", "unlisted", "private"] as const;

/** The tag installers follow when they request no version. */
export const LATEST_TAG = "latest";

/** A distribution tag name: lowercase, starting with a letter, as long as the longest version. */
const tagName = schema
    .string()
    .max(TAG_LENGTH)
    .regex(/^[a-z][a-z0-9._-]*$/u);

/** A distribution tag name, which no version range reads as a range. */
export const TagName = Object.assign(tagName, {
    /** Require a tag name that no version range reads as a range. */
    require(name: string): void {
        if (validRange(name) !== null) {
            throw RegistryError.of(
                "TAG_INVALID",
                `distribution tag '${name}' is invalid: it must not be a version range`,
            );
        }
    },
});

/** A pushed lock checked for its release, before the release's transaction. */
export const Candidate = schema.object({
    /** The digest of the lock. */
    lock: Digest,
    /** The commit its root compiled, absent for a prerelease of uncommitted changes. */
    commit: schema.string().exactOptional(),
    /** The version its root build declares. */
    version: schema.string(),
    /** Its root build's declarations, with their terms. */
    declarations: schema.array(graph.Declaration).readonly(),
});
/** A pushed lock checked for its release, before the release's transaction. */
export type Candidate = schema.Infer<typeof Candidate>;

/** A package of an account, installed as `@<account handle>/<name>` and keyed by its package id. */
export const packageObject = defineObject(
    {
        name: "package",
        plural: "packages",
        title: t`${plural(argument("count"), { one: "Package", other: "Packages" })}`,
        scope: space,
        owner: "account",
        fields: {
            /** The account publishing the package. */
            account: field.reference(account).relate("account"),
            /** The npm package name within the account's scope. */
            name: field.string(
                schema
                    .string()
                    .max(NAME_LENGTH)
                    .regex(/^[a-z0-9][a-z0-9._-]*$/u),
            ),
            /** Who can read the package, and whether it is listed to them. */
            visibility: field.enum(VISIBILITIES).default("unlisted"),
            /** The terms of the package's releases. */
            vocabulary: field.json(Vocabulary).default({}),
        },
        indexes: {
            name: {
                on: ["accountId", "name"],
                across: () => universe,
            },
            id: { on: ["id"], across: () => universe },
        },
        constraints: {
            name: {
                kind: "check",
                condition: {
                    name: {
                        length: { gte: 1, lte: NAME_LENGTH },
                        characters: "a-z0-9._-",
                        NOT: {
                            OR: [{ startsWith: "." }, { startsWith: "_" }, { startsWith: "-" }],
                        },
                    },
                },
            },
        },
        access: {
            attributes: { visibility: "string" },
            permissions: {
                read: union(
                    resource({ visibility: { in: ["public", "unlisted"] } }),
                    through("account", "read"),
                ),
                discover: resource({ visibility: "public" }),
                publish: through("account", "publish"),
            },
        },
    },
    (method) => ({
        create: method.create("publish", {
            fields: ["accountId", "name", "visibility"],
            onConflict: "id",
        }),
        /** Publish a pushed lock as a release under a tag, creating the package, as npm does. */
        publish: method.mutation({
            title: t`Publish`,
            permission: "publish",
            target: "scope",
            prepared: Candidate,
            input: schema.object({
                /** The package. */
                packageId: PackageId,
                /** The account publishing it. */
                accountId: schema.identifier("account"),
                /** Its name within the account. */
                name: schema.string(),
                /** Who can read it, unlisted on its first publish when absent. */
                visibility: schema.enum(VISIBILITIES).exactOptional(),
                /** The digest of the pushed lock. */
                lock: Digest,
                /** The commit its root compiled, absent for a prerelease of uncommitted changes. */
                commit: schema.string().exactOptional(),
                /** The distribution tag pointing at the release, latest when absent. */
                tag: TagName.exactOptional(),
            }),
        }),
        list: method.list("discover"),
        /** Find the package of a global name, `@<account handle>/<name>`. */
        resolve: method.query({
            title: t`Resolve`,
            permission: "read",
            target: "scope",
            input: schema.object({
                /** The global name, such as @acme/notebooks. */
                name: PackageName,
            }),
            prepared: ObjectReference,
        }),
        /** Point a distribution tag at a published version, creating the tag the first time. */
        point: method.mutation({
            title: t`Point`,
            permission: "publish",
            input: schema.object({
                /** The distribution tag. */
                name: TagName,
                /** The published version. */
                version: schema.string(),
            }),
        }),
        advance: method.update("publish", { visibility: "package", fields: ["vocabulary"] }),
        update: method.update("publish", { fields: ["visibility"] }),
    }),
);

/** A persisted package record. */
export type Package = Select<typeof packageObject.table>;
