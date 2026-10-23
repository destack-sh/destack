import { schema } from "@destack/schema";

/** A file's metadata as JSON: times as ISO strings, digests as hexadecimal text. */
export const FileDescription = schema.object({
    /** The exact file key. */
    key: schema.string(),
    /** The unique upload identifier. */
    version: schema.string(),
    /** The complete file length in bytes. */
    size: schema.number(),
    /** The unquoted entity tag. */
    etag: schema.string(),
    /** The upload time. */
    uploaded: schema.string(),
    /** The stored HTTP headers, the expiry as an ISO time. */
    httpMetadata: schema.record(schema.string(), schema.string()),
    /** Application metadata. */
    customMetadata: schema.record(schema.string(), schema.string()),
    /** The stored hexadecimal digests by algorithm. */
    digest: schema.record(schema.string(), schema.string()),
    /** The storage class. */
    storageClass: schema.enum(["Standard", "InfrequentAccess"]),
    /** The MD5 digest of the customer key encrypting the file, as base64. */
    ssecKeyMd5: schema.string().exactOptional(),
    /** The time before which the file can be neither replaced nor deleted. */
    retainUntil: schema.string().exactOptional(),
});
/** A file's metadata as JSON. */
export type FileDescription = schema.Infer<typeof FileDescription>;
