import { check, index, integer, json, primaryKey, sql, defineTable, text } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { schema } from "@destack/schema";
import { CHECKSUM_ALGORITHMS, STORAGE_CLASSES } from "../../bucket/index.ts";

/** Stored HTTP metadata with textual expiration times. */
const HttpMetadata = schema.object({
    contentType: schema.string().optional(),
    contentLanguage: schema.string().optional(),
    contentDisposition: schema.string().optional(),
    contentEncoding: schema.string().optional(),
    cacheControl: schema.string().optional(),
    cacheExpiry: schema.string().optional(),
});

/** The currently published file at each key. */
export const file = defineTable("file", {
    /** The caller's UTF-8 key. */
    key: text("key").primaryKey().notNull(),
    /** The unique upload identifier, the name of its first content file unless copied. */
    version: text("version").notNull(),
    /** The entity tag. */
    etag: text("etag").notNull(),
    /** The stored content checksums. */
    checksums: json(
        "checksums",
        schema.partialRecord(schema.enum(CHECKSUM_ALGORITHMS), schema.string()),
    ).notNull(),
    /** The stored byte count. */
    size: integer("size").notNull(),
    /** The publication time. */
    uploaded: integer("uploaded").notNull(),
    /** HTTP headers, with expiration encoded as an ISO date. */
    httpMetadata: json("http_metadata", HttpMetadata).notNull(),
    /** Application metadata. */
    customMetadata: json(
        "custom_metadata",
        schema.record(schema.string(), schema.string()),
    ).notNull(),
    /** The storage class. */
    storageClass: text("storage_class", { enum: STORAGE_CLASSES }).notNull(),
    /** The base64 MD5 digest of the customer key encrypting the content, absent for plain content. */
    ssecKeyMd5: text("ssec_key_md5"),
});

/** The ordered immutable content files of each file version; copies share their source's. */
export const segment = defineTable(
    "segment",
    {
        /** The file version. */
        version: text("version").notNull(),
        /** The position within the file, from 0. */
        position: integer("position").notNull(),
        /** The immutable content filename. */
        content: text("content").notNull(),
        /** The content length. */
        size: integer("size").notNull(),
    },
    {
        constraints: (table) => [
            primaryKey({ columns: [table.version, table.position] }),
            index("segment_content").on(table.content),
        ],
    },
);

/** An incomplete multipart upload and its destination metadata. */
export const upload = defineTable(
    "upload",
    {
        /** The unique upload identifier. */
        id: text("id").primaryKey().notNull(),
        /** The destination key. */
        key: text("key").notNull(),
        /** The upload lifecycle. */
        state: text("state", { enum: ["active", "completed", "aborted"] }).notNull(),
        /** The automatic cleanup time. */
        expires: integer("expires").notNull(),
        /** The storage class of the completed file. */
        storageClass: text("storage_class", { enum: STORAGE_CLASSES }).notNull(),
        /** The base64 MD5 digest of the customer key encrypting the parts, absent for plain content. */
        ssecKeyMd5: text("ssec_key_md5"),
        /** HTTP and application metadata, with expiration encoded as an ISO date. */
        options: json(
            "options",
            schema.object({
                httpMetadata: HttpMetadata.optional(),
                customMetadata: schema.record(schema.string(), schema.string()).optional(),
            }),
        ).notNull(),
    },
    {
        constraints: (table) => [
            index("upload_expiration").on(table.expires),
            check("upload_state", sql`${table.state} IN ('active', 'completed', 'aborted')`),
        ],
    },
);

/** The current immutable file for each numbered upload part. */
export const part = defineTable(
    "part",
    {
        /** The upload identifier. */
        uploadId: text("upload_id").notNull(),
        /** The part number. */
        partNumber: integer("part_number").notNull(),
        /** The immutable content filename. */
        content: text("content").notNull(),
        /** The content length. */
        size: integer("size").notNull(),
        /** The part entity tag. */
        etag: text("etag").notNull(),
        /** The part MD5 digest used to compute the completed file tag. */
        md5: text("md5").notNull(),
        /** The upload time. */
        uploaded: integer("uploaded").notNull(),
    },
    {
        constraints: (table) => [
            primaryKey({ columns: [table.uploadId, table.partNumber] }),
            index("part_content").on(table.content),
        ],
    },
);

/** The private catalogue database of a local bucket. */
export const catalogueDatabase = defineDatabase({
    name: "catalogue",
    tables: [file, segment, upload, part],
});
