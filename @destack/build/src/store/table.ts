import {
    BLOB_TABLES,
    blob,
    defineDatabase,
    defineTable,
    index,
    integer,
    json,
    text,
} from "@destack/db";
import { Digest } from "@destack/schema";
import { CacheEntry } from "../cache/entry.ts";

/** The builds a store has, each present once every blob it references is stored. */
export const buildManifest = defineTable("manifest", {
    /** The digest of the manifest's bytes, a blob the record goes with. */
    digest: blob("digest", { onDelete: "cascade" }).primaryKey(),
    /** The manifest's byte count. */
    size: integer("size").notNull(),
});

/** The blobs each build's manifest references: its lists, files and dependency builds. */
export const buildReference = defineTable(
    "reference",
    {
        /** The build, whose deletion takes the reference. */
        manifest: blob("manifest", { onDelete: "cascade" }).primaryKey(),
        /** The referenced blob, a file or another build's manifest. */
        digest: blob("digest").primaryKey(),
    },
    { constraints: (table) => [index("reference_digest").on(table.digest)] },
);

/** The blobs attached to builds at paths, as OCI's referrers attach artifacts to a subject. */
export const buildReferrer = defineTable(
    "referrer",
    {
        /** The build the blob is attached to, whose deletion takes the attachment. */
        subject: blob("subject", { onDelete: "cascade" }).primaryKey(),
        /** The attachment's path, such as `runtime/bun` or `maps`. */
        path: text("path").primaryKey(),
        /** The attached blob. */
        digest: blob("digest").notNull(),
        /** The attached blob's byte count. */
        size: integer("size").notNull(),
        /** The attached blob's media type. */
        mediaType: text("media_type").notNull(),
    },
    { constraints: (table) => [index("referrer_digest").on(table.digest)] },
);

/** The blobs each root retains: a space's builds, a directory's, a cache entry's. */
export const buildRoot = defineTable(
    "root",
    {
        /** The root's name. */
        name: text("name").primaryKey(),
        /** The retained blob, a build's manifest or a file. */
        digest: blob("digest").primaryKey(),
    },
    { constraints: (table) => [index("root_digest").on(table.digest)] },
);

/** The build cache's entries by key, each a root of what it references while used. */
export const buildCache = defineTable(
    "cache",
    {
        /** The cache key's digest. */
        key: text("key").validate(Digest).primaryKey(),
        /** What the key stores. */
        entry: json("entry", CacheEntry).notNull(),
        /** When the entry was last stored or read, in UTC epoch milliseconds. */
        usedAt: integer("used_at").notNull(),
    },
    { constraints: (table) => [index("cache_used_at").on(table.usedAt)] },
);

/** The tables of a package store's index, which its owner declares in its database. */
export const PACKAGE_STORE_TABLES = [
    buildManifest,
    buildReference,
    buildReferrer,
    buildRoot,
    buildCache,
];

/** The database of a store of its own, such as a toolchain's on its machine's disk. */
export const packageStoreDatabase = defineDatabase({
    name: "builds",
    tables: [...PACKAGE_STORE_TABLES, ...BLOB_TABLES],
});
