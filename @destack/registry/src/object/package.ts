import { condition, none, through } from "@destack/access";
import { account } from "@destack/account/object";
import {
    check,
    dialectSQL,
    foreignKey,
    index,
    sql,
    unique,
    type Select,
    type TableConstraint,
} from "@destack/db";
import { Condition } from "@destack/db/query";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { Upgrade, Vocabulary } from "@destack/resource";

/**
 * The longest package name within an account.
 *
 * npm names are at most 214 characters: 214 minus a 63 character handle minus `@` and `/` is 149.
 */
const NAME_LENGTH = 149;

/** The longest deprecation message, about a paragraph of text. */
const DEPRECATION_LENGTH = 1024;

/** The longest distribution tag name, as long as the longest version it may stand for. */
const TAG_LENGTH = 256;

/** The tag installers follow when they name no version. */
export const LATEST_TAG = "latest";

/** A distribution tag name: lowercase, starting with a letter, as long as the longest version. */
export const TagName = schema
    .string()
    .max(TAG_LENGTH)
    .regex(/^[a-z][a-z0-9._-]*$(?![\s\S])/);

/** The warning installers show for a deprecated release. */
const DeprecationMessage = schema.string().min(1).max(DEPRECATION_LENGTH);

/** The npm archive installers download for a release. */
export const Distribution = schema.object({
    /** The archive's SHA-256 digest and storage key. */
    digest: schema.string(),
    /** The archive's SHA-1 checksum for npm clients. */
    shasum: schema.string(),
    /** The archive's SHA-512 Subresource Integrity expression. */
    integrity: schema.string(),
    /** The compressed size in bytes. */
    size: schema.number().int(),
    /** The total uncompressed size in bytes. */
    unpackedSize: schema.number().int(),
    /** The number of archived regular files. */
    fileCount: schema.number().int(),
});
/** The npm archive installers download for a release. */
export type Distribution = schema.Infer<typeof Distribution>;

/** The npm declaration of a release, generated from its build. */
export const PackageMetadata = schema.object({
    /** The scoped package name. */
    name: schema.string(),
    /** The immutable release version. */
    version: schema.string(),
    /** Compiled outputs use ECMAScript modules. */
    type: schema.literal("module"),
    /** Conditional compiled entrypoints in priority order. */
    exports: schema.record(schema.string(), schema.record(schema.string(), schema.string())),
    /** Exact required dependency versions and npm aliases. */
    dependencies: schema.record(schema.string(), schema.string()),
    /** Exact optional dependency versions and npm aliases. */
    optionalDependencies: schema.record(schema.string(), schema.string()),
    /** Shared dependency version ranges supplied by consumers. */
    peerDependencies: schema.record(schema.string(), schema.string()).optional(),
    /** Optional peer declarations. */
    peerDependenciesMeta: schema
        .record(
            schema.string(),
            schema.object({
                optional: schema.boolean().optional(),
            }),
        )
        .optional(),
    /** Authored package description. */
    description: schema.string().optional(),
    /** The commit the release was built from, as npm records it. */
    gitHead: schema.string(),
    /** Authored SPDX license expression. */
    license: schema.string().optional(),
    /** Whether importing a module can have side effects. */
    sideEffects: schema.boolean().optional(),
});
/** The npm declaration of a release, generated from its build. */
export type PackageMetadata = schema.Infer<typeof PackageMetadata>;

/** A package of an account, installed as `@<account handle>/<name>`. */
export const packageObject = defineObject({
    name: "package",
    plural: "packages",
    scope: account,
    fields: {
        /** The npm package name within the account's scope. */
        name: field.string(
            schema
                .string()
                .max(NAME_LENGTH)
                .regex(/^[a-z0-9][a-z0-9._-]*$(?![\s\S])/),
        ),
        /** Who can read the package, and whether it is listed to them. */
        visibility: field.enum(["public", "unlisted", "private"]).default("unlisted"),
        /** The terms of the package's releases. */
        vocabulary: field.json(Vocabulary).default({}),
    },
    attributes: { visibility: "string" },
    indexes: { name: { on: ["name"], unique: true, across: account } },
    constraints: (entry) => [
        check(
            "package_name",
            dialectSQL({
                sqlite: sql`length(${entry.name}) BETWEEN 1 AND ${sql.raw(String(NAME_LENGTH))} AND substr(${entry.name}, 1, 1) NOT IN ('.', '_', '-') AND ${entry.name} NOT GLOB '*[^a-z0-9._-]*'`,
                postgresql: sql`(${entry.name} COLLATE "C") ~ ${sql.raw(`'^[a-z0-9][a-z0-9._-]{0,${NAME_LENGTH - 1}}$'`)}`,
            }),
        ),
    ],
    permissions: {
        read: condition(Condition.oneOf("visibility", ["public", "unlisted"])),
        discover: condition(Condition.eq("visibility", "public")),
        publish: none(),
    },
    methods: {
        get: method.get("read"),
        list: method.list("discover"),
        create: method.create("publish", { fields: ["name", "visibility"] }),
        update: method.update("publish", { fields: ["visibility"] }),
        advance: method.update(null, { isSystem: true, fields: ["vocabulary"] }),
    },
});

/**
 * An immutable release of a package at one calendar version, built from one commit of the package's source.
 *
 * Unpublishing keeps the release as a tombstone to block its version forever.
 */
export const release = defineObject({
    name: "release",
    plural: "releases",
    scope: account,
    nested: { in: packageObject, receive: "publish", delete: "restrict" },
    fields: {
        /** The immutable calendar version. */
        version: field.string(),
        /** The SHA-256 digest of the published manifest. */
        manifest: field.string(),
        /** The full Git SHA-1 or SHA-256 commit identifier. */
        commit: field.string(),
        /** The npm archive installers download. */
        distribution: field.json(Distribution),
        /** The npm declaration installers read. */
        metadata: field.json(PackageMetadata),
        /** The steps from the package's previous release, absent for its first. */
        upgrade: field.json(Upgrade).optional(),
        /** The warning installers show for the release, absent while it is not deprecated. */
        deprecation: field.string(DeprecationMessage).optional(),
        /** When the release was unpublished. */
        unpublishedAt: field.time().optional(),
    },
    constraints: (release) => [
        check(
            "release_commit",
            dialectSQL({
                sqlite: sql`length(${release.commit}) IN (40, 64) AND ${release.commit} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${release.commit}) IN (40, 64) AND (${release.commit} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
        check(
            "release_manifest",
            dialectSQL({
                sqlite: sql`length(${release.manifest}) = 64 AND ${release.manifest} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${release.manifest}) = 64 AND (${release.manifest} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
        unique("release_version").on(release.parentId, release.version),
    ],
    permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("publish", {
            isPredicted: false,
            fields: ["commit", "manifest"],
            input: schema.object({
                /** The distribution tag pointing at the release, latest when absent. */
                tag: TagName.optional(),
            }),
        }),
        deprecate: method({
            permission: "publish",
            input: schema.object({
                /** The warning installers show, or null to withdraw the deprecation. */
                message: DeprecationMessage.nullable(),
            }),
        }).handle((call) => call.revise({ deprecation: call.input.message })),
        unpublish: method({ permission: "publish" }),
    },
});

/** An npm distribution tag of a package, such as latest, pointing at one published version. */
export const tag = defineObject({
    name: "tag",
    plural: "tags",
    scope: account,
    nested: { in: packageObject, receive: "publish", delete: "restrict" },
    fields: {
        /** The npm distribution tag name, invalid as a version range. */
        name: field.string(TagName),
        /** The selected published version. */
        version: field.string(),
    },
    constraints: (entry): TableConstraint[] => [
        unique("tag_name").on(entry.parentId, entry.name),
        foreignKey({
            columns: [entry.parentId, entry.version],
            foreignColumns: [release.table.parentId, release.table.version],
        }).onDelete("restrict"),
    ],
    permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
    methods: {
        list: method.list("read"),
        create: method.create("publish", { fields: ["name", "version"] }),
        update: method.update("publish", { fields: ["version"] }),
        delete: method.delete("publish"),
        point: method.create(null, { isSystem: true, fields: ["name", "version"] }),
    },
});

/** An exact release another release requires, which blocks unpublishing it. */
export const dependency = defineObject({
    name: "dependency",
    plural: "dependencies",
    scope: account,
    nested: { in: release, receive: "publish" },
    fields: {
        /** The required package's npm name. */
        name: field.string(),
        /** The required exact version. */
        version: field.string(),
    },
    constraints: (entry) => [index("dependency_release").on(entry.name, entry.version)],
    permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
    methods: {
        create: method.create(null, { isSystem: true, fields: ["name", "version"] }),
    },
});

/** A persisted package record. */
export type Package = Select<typeof packageObject.table>;
/** A persisted release record. */
export type Release = Select<typeof release.table>;
/** A persisted tag record. */
export type Tag = Select<typeof tag.table>;
