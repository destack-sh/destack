import { resource, through, union, universe } from "@destack/access";
import { account, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { PackageId, PackageName } from "@destack/package";
import { Vocabulary } from "@destack/resource";
import { CallError } from "@destack/object";
import { validRange } from "semver";

/** The longest package name in an account: npm's 214 minus a 63 character handle, `@` and `/`. */
const NAME_LENGTH = 149;

/** The longest distribution tag name, as long as the longest version it may stand for. */
const TAG_LENGTH = 256;

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
            throw new CallError("BAD_REQUEST", { message: `invalid distribution tag: ${name}` });
        }
    },
});

/** A package of an account, installed as `@<account handle>/<name>` and keyed by its package id. */
export const packageObject = defineObject(
    {
        name: "package",
        plural: "packages",
        scope: space,
        owner: "account",
        key: PackageId,
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
            visibility: field.enum(["public", "unlisted", "private"]).default("unlisted"),
            /** The terms of the package's releases. */
            vocabulary: field.json(Vocabulary).default({}),
        },
        attributes: { visibility: "string" },
        indexes: {
            name: {
                on: ["accountId", "name"],
                across: () => universe,
            },
            id: { on: ["id"], across: () => universe },
        },
        permissions: {
            read: union(
                resource({ visibility: { in: ["public", "unlisted"] } }),
                through("account", "read"),
            ),
            discover: resource({ visibility: "public" }),
            publish: through("account", "manage"),
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
    },
    (method) => ({
        create: method.create("publish", { fields: ["accountId", "name", "visibility"] }),
        list: method.list("discover"),
        /** Find the package of a global name, `@<account handle>/<name>`. */
        resolve: method.query({
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
            permission: "publish",
            isInternal: true,
            input: schema.object({
                /** The distribution tag. */
                name: TagName,
                /** The published version. */
                version: schema.string(),
            }),
        }),
        advance: method.update("publish", { isInternal: true, fields: ["vocabulary"] }),
        update: method.update("publish", { fields: ["visibility"] }),
    }),
);

/** A persisted package record. */
export type Package = Select<typeof packageObject.table>;
