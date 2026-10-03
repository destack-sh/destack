/** Stored metadata fields and their HTTP header names. */
export const HTTP_METADATA_FIELDS = [
    ["contentType", "content-type"],
    ["contentLanguage", "content-language"],
    ["contentDisposition", "content-disposition"],
    ["contentEncoding", "content-encoding"],
    ["cacheControl", "cache-control"],
] as const satisfies readonly (readonly [keyof BucketHttpMetadata, string])[];

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

/** Stored HTTP metadata: written as headers, and encoded as strings for transfers. */
export const BucketHttpMetadata = {
    /** Write the metadata as the headers a file is served or uploaded with. */
    write(metadata: BucketHttpMetadata, headers: Headers): void {
        // write only the headers the metadata has
        for (const [field, header] of HTTP_METADATA_FIELDS) {
            const value = metadata[field];
            if (value !== undefined) {
                headers.set(header, value);
            }
        }
        if (metadata.cacheExpiry !== undefined) {
            headers.set("expires", metadata.cacheExpiry.toUTCString());
        }
    },

    /** Encode the metadata as strings, its expiry as an ISO time. */
    encode(metadata: BucketHttpMetadata): Record<string, string> {
        // encode each header field the metadata has, then its expiry
        const encoded: Record<string, string> = {};
        for (const [field] of HTTP_METADATA_FIELDS) {
            const value = metadata[field];
            if (value !== undefined) {
                encoded[field] = value;
            }
        }
        if (metadata.cacheExpiry !== undefined) {
            encoded["cacheExpiry"] = metadata.cacheExpiry.toISOString();
        }

        return encoded;
    },

    /** Decode metadata encoded as strings. */
    decode(encoded: Readonly<Record<string, string>>): BucketHttpMetadata {
        const { cacheExpiry, ...rest } = encoded;

        return {
            ...rest,
            ...(cacheExpiry === undefined ? {} : { cacheExpiry: new Date(cacheExpiry) }),
        };
    },
};
