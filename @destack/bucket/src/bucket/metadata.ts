/** Stored metadata fields and their HTTP header names. */
export const HTTP_METADATA_FIELDS = {
    contentType: "content-type",
    contentLanguage: "content-language",
    contentDisposition: "content-disposition",
    contentEncoding: "content-encoding",
    cacheControl: "cache-control",
} as const;

/** HTTP headers stored with a file. */
export interface BucketHttpMetadata {
    /** The media type. */
    contentType?: string;
    /** The content language. */
    contentLanguage?: string;
    /** The download disposition and filename. */
    contentDisposition?: string;
    /** The content encoding. */
    contentEncoding?: string;
    /** The cache policy. */
    cacheControl?: string;
    /** The expiration time. */
    cacheExpiry?: Date;
}
