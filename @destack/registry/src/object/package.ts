import { resource, through, union, universe } from "@destack/access";
import { account, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Digest, schema, Version } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import {
    graph,
    PACKAGE_ROLES,
    PackageName,
    PackageRelease,
    PackageRole,
    PackageVersion,
} from "@destack/package";
import { Vocabulary } from "@destack/resource";
import { RegistryError } from "../error/index.ts";
import { NpmRelease } from "../npm/packument.ts";
import { validRange } from "semver";
import { argument, plural, t } from "@destack/locale";

/** The longest package name in an account: npm's 214 minus a 63 character handle, `@` and `/`. */
const NAME_LENGTH = 149;

/** The longest distribution tag name, as long as the longest version it may stand for. */
const TAG_LENGTH = 256;

/** Who can read a package: anyone, anyone with its name, or its account alone. */
const VISIBILITIES = ["public", "unlisted", "private"] as const;

/** Who can read a package and whether it is listed to them. */
export type PackageVisibility = (typeof VISIBILITIES)[number];

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

/** A pushed image checked for its release, before the release's transaction. */
export const Candidate = schema.object({
    /** The digest of the image. */
    image: Digest,
    /** The commit its root compiled. */
    commit: schema.string(),
    /** The version its root build declares. */
    version: schema.string(),
    /** The role its root build declares: an app, a library or a builtin. */
    role: PackageRole,
    /** Its root build's declarations, with their terms. */
    declarations: schema.array(graph.Declaration).readonly(),
    /** What an npm client lists of the release: install fields and tarball hash. */
    npm: NpmRelease,
    /** The digest of the stored tarball npm clients download. */
    tarball: Digest,
});
/** A pushed image checked for its release, before the release's transaction. */
export type Candidate = schema.Infer<typeof Candidate>;

/** A package a publish releases, checked before its transaction: the package and its release. */
const Publication = schema.object({
    /** The package and the version its build declares. */
    package: PackageVersion,
    /** Who can read the package as its package file asks through npm's access, if it asks. */
    visibility: schema.enum(VISIBILITIES).exactOptional(),
    /** The package's image checked for its release. */
    release: Candidate,
});

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
            /** The role its last published release declares: an app, a library or a builtin. */
            role: field.enum(PACKAGE_ROLES),
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
            fields: ["accountId", "name", "visibility", "role"],
            onConflict: { key: "id" },
        }),
        /** Publish an account's packages in a pushed image under a tag, required ones first. */
        publish: method.mutation({
            title: t.context("action")`Publish`,
            permission: "publish",
            target: "scope",
            prepared: schema.array(Publication).readonly(),
            input: schema.object({
                /** The account publishing them. */
                accountId: schema.identifier("account"),
                /** Who can read each, else as its package file asks, else unlisted at first. */
                visibility: schema.enum(VISIBILITIES).exactOptional(),
                /** The digest of the pushed image. */
                image: Digest,
                /** The commit its root compiled, which every release is built from. */
                commit: schema.string(),
                /** The distribution tag pointing at each release, latest when absent. */
                tag: TagName.exactOptional(),
                /** The further distribution tags to point at each release, as npm dist-tag add. */
                tags: schema.array(TagName).readonly().exactOptional(),
            }),
            output: schema.object({
                /** The packages' releases, in the order they published. */
                releases: schema.array(PackageRelease).readonly(),
            }),
        }),
        list: method.list("discover"),
        /** Find the package of a global name, `@<account handle>/<name>`. */
        resolve: method.query({
            title: t.context("action")`Resolve`,
            permission: "read",
            target: "scope",
            input: schema.object({
                /** The global name, such as @example/notebooks. */
                name: PackageName,
            }),
            prepared: ObjectReference,
        }),
        /** Read the version the next release of a package name takes, published yet or not. */
        next: method.query({
            title: t.context("action")`Next`,
            permission: "read",
            target: "scope",
            input: schema.object({
                /** The package's global name, such as @example/notebooks. */
                name: PackageName,
            }),
            output: schema.object({
                /** The version the next release takes. */
                version: Version,
            }),
        }),
        /** Point a distribution tag at a published version, creating the tag the first time. */
        point: method.mutation({
            title: t.context("action")`Point`,
            permission: "publish",
            input: schema.object({
                /** The distribution tag. */
                name: TagName,
                /** The published version. */
                version: schema.string(),
            }),
        }),
        advance: method.update("publish", {
            visibility: "package",
            fields: ["vocabulary", "role"],
        }),
        update: method.update("publish", { fields: ["visibility"] }),
    }),
);

/** A persisted package record. */
export type Package = Select<typeof packageObject.table>;
