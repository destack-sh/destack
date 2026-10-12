import {
    BucketCustomMetadata,
    BucketFile,
    type BucketHttpMetadata,
    ContentDigest,
    type DigestAlgorithm,
    HTTP_METADATA_FIELDS,
    MAX_CUSTOM_METADATA_BYTES,
    STORAGE_CLASSES,
    type StorageClass,
} from "../bucket/index.ts";
import { S3Error } from "./error.ts";
import { md5 } from "./md5.ts";

/** The length of a customer-provided AES-256 key in bytes. */
const CUSTOMER_KEY_BYTES = 32;

/** The headers of customer-provided encryption keys. */
export const CUSTOMER_KEY_HEADERS = {
    algorithm: "x-amz-server-side-encryption-customer-algorithm",
    key: "x-amz-server-side-encryption-customer-key",
    md5: "x-amz-server-side-encryption-customer-key-md5",
} as const;

/** The S3 name of each storage class, as R2 maps them. */
export const S3_STORAGE_CLASSES: Record<StorageClass, string> = {
    Standard: "STANDARD",
    InfrequentAccess: "STANDARD_IA",
};

/** The digests S3 carries as checksum headers, by their header. */
export const CHECKSUM_HEADERS = [
    ["sha1", "x-amz-checksum-sha1"],
    ["sha256", "x-amz-checksum-sha256"],
] as const satisfies readonly (readonly [DigestAlgorithm, string])[];

/** The header asking a read to answer the file's checksums. */
export const CHECKSUM_MODE_HEADER = "x-amz-checksum-mode";

/** The header naming the stored version of a file. */
const VERSION_HEADER = "x-amz-version-id";

/** The prefix of user metadata headers. */
const METADATA_PREFIX = "x-amz-meta-";

/** A single-part entity tag, which S3 and R2 compute as the body's MD5 digest. */
const MD5_ETAG = /^[0-9a-f]{32}$/u;

/** The S3 headers describing a file: written by the server, read by the client. */
export const S3Headers = {
    write,
    read,
    checksums,
    httpMetadata,
    customMetadata,
    retainUntil,
    storageClass,
    storageClassNamed,
};

/** Describe a file in response headers, with its checksums when a read asks for them. */
function write(file: BucketFile, isChecksummed: boolean): Headers {
    // describe the version and its stored HTTP metadata
    const headers = new Headers({
        etag: file.httpEtag,
        "last-modified": file.uploaded.toUTCString(),
        "accept-ranges": "bytes",
        [VERSION_HEADER]: file.version,
    });
    file.writeHttpMetadata(headers);

    // add user metadata, a non-standard storage class, the object lock and the customer key digest
    for (const [name, value] of Object.entries(file.customMetadata)) {
        headers.set(`${METADATA_PREFIX}${name}`, value);
    }
    if (file.storageClass !== "Standard") {
        headers.set("x-amz-storage-class", S3_STORAGE_CLASSES[file.storageClass]);
    }
    if (file.retainUntil !== undefined) {
        headers.set("x-amz-object-lock-mode", "COMPLIANCE");
        headers.set("x-amz-object-lock-retain-until-date", file.retainUntil.toISOString());
    }
    if (file.ssecKeyMd5 !== undefined) {
        headers.set(CUSTOMER_KEY_HEADERS.algorithm, "AES256");
        headers.set(CUSTOMER_KEY_HEADERS.md5, file.ssecKeyMd5);
    }

    // add the stored checksums a read asks for
    if (isChecksummed) {
        for (const [name, value] of Object.entries(checksums(file))) {
            headers.set(name, value);
        }
    }

    return headers;
}

/** Read the file a response describes, of a size the caller reads from the response. */
function read(key: string, size: number, headers: Headers): BucketFile {
    // require the version, entity tag and upload time every file answer carries
    const version = headers.get(VERSION_HEADER);
    const etag = headers.get("etag");
    const modified = headers.get("last-modified");
    if (version === null || etag === null || modified === null) {
        throw new TypeError(`the answer for ${key} describes no file version`);
    }
    const unquoted = etag.replace(/^"(.*)"$/u, "$1");

    // read the stored metadata, lock and customer key digest
    const ssecKeyMd5 = headers.get(CUSTOMER_KEY_HEADERS.md5);

    return new BucketFile(
        key,
        version,
        size,
        unquoted,
        new Date(modified),
        httpMetadata(headers),
        customMetadata(headers),
        digest(unquoted, headers),
        storageClass(headers) ?? "Standard",
        ssecKeyMd5 ?? undefined,
        retainUntil(headers),
    );
}

/** Write the checksum headers of a file's stored digests, in base64. */
function checksums(file: BucketFile): Record<string, string> {
    const written: Record<string, string> = {};
    for (const [algorithm, name] of CHECKSUM_HEADERS) {
        const value = file.digest[algorithm];
        if (value !== undefined) {
            written[name] = new Uint8Array(value).toBase64();
        }
    }

    return written;
}

/** Read a file's digests. */
function digest(etag: string, headers: Headers): ContentDigest {
    // take a single-part entity tag as the MD5 digest
    const digests: Partial<Record<DigestAlgorithm, string>> = MD5_ETAG.test(etag)
        ? { md5: etag }
        : {};

    // take each checksum header as hexadecimal
    for (const [algorithm, name] of CHECKSUM_HEADERS) {
        const value = headers.get(name);
        if (value !== null) {
            digests[algorithm] = Uint8Array.fromBase64(value).toHex();
        }
    }

    return new ContentDigest(digests);
}

/** Read the stored HTTP headers, without the aws-chunked content encoding. */
function httpMetadata(headers: Headers): BucketHttpMetadata {
    // copy the supported header fields
    const metadata: BucketHttpMetadata = {};
    for (const [field, name] of HTTP_METADATA_FIELDS) {
        const value = headers.get(name);
        if (value !== null) {
            metadata[field] = value;
        }
    }

    // drop the transfer coding S3 removes before storing
    const encoding = metadata.contentEncoding
        ?.split(",")
        .map((entry) => entry.trim())
        .filter((entry) => entry !== "aws-chunked")
        .join(", ");
    if (encoding === "") {
        delete metadata.contentEncoding;
    } else if (encoding !== undefined) {
        metadata.contentEncoding = encoding;
    }

    // require a valid expiration date
    const expires = headers.get("expires");
    if (expires !== null) {
        const time = Date.parse(expires);
        if (!Number.isFinite(time)) {
            throw new S3Error("InvalidArgument", "the expires header must be an HTTP date");
        }
        metadata.cacheExpiry = new Date(time);
    }

    return metadata;
}

/** Read the user metadata, within S3's two KiB limit. */
function customMetadata(headers: Headers): Record<string, string> {
    // collect the x-amz-meta headers and measure their names and values
    const metadata: Record<string, string> = {};
    for (const [name, value] of headers) {
        if (name.startsWith(METADATA_PREFIX)) {
            metadata[name.slice(METADATA_PREFIX.length)] = value;
        }
    }

    // refuse metadata beyond the limit
    if (BucketCustomMetadata.bytes(metadata) > MAX_CUSTOM_METADATA_BYTES) {
        throw new S3Error("MetadataTooLarge", "user metadata exceeds the 2 KiB limit");
    }

    return metadata;
}

/** Read the retain-until date of a compliance-mode object lock, else undefined. */
function retainUntil(headers: Headers): Date | undefined {
    // read the optional headers, accepting the compliance mode alone
    const value = headers.get("x-amz-object-lock-retain-until-date");
    const mode = headers.get("x-amz-object-lock-mode");
    if (value === null) {
        return undefined;
    } else if (mode !== null && mode !== "COMPLIANCE") {
        throw new S3Error("InvalidArgument", "object lock supports the COMPLIANCE mode alone");
    }

    // read the ISO 8601 date
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
        throw new S3Error("InvalidArgument", `invalid object lock retain-until date: ${value}`);
    }

    return date;
}

/** Read the storage class, or undefined for the default. */
function storageClass(headers: Headers): StorageClass | undefined {
    const value = headers.get("x-amz-storage-class");

    return value === null ? undefined : storageClassNamed(value);
}

/** Read the storage class S3 names, refusing a class this server does not offer. */
function storageClassNamed(name: string): StorageClass {
    const found = STORAGE_CLASSES.find((entry) => S3_STORAGE_CLASSES[entry] === name);
    if (found === undefined) {
        throw new S3Error("InvalidStorageClass", `the storage class ${name} is not supported`);
    }

    return found;
}

/** Write the SSE-C headers that send a base64 customer key with a request. */
export function customerKeyHeaders(key: string): Record<string, string> {
    return {
        [CUSTOMER_KEY_HEADERS.algorithm]: "AES256",
        [CUSTOMER_KEY_HEADERS.key]: key,
        [CUSTOMER_KEY_HEADERS.md5]: md5(readCustomerKey(key)).toBase64(),
    };
}

/** Decode a customer key, which must be 32 bytes of base64. */
export function readCustomerKey(value: string): Uint8Array {
    // decode the key, refusing malformed base64
    let bytes: Uint8Array;
    try {
        bytes = Uint8Array.fromBase64(value);
    } catch (cause) {
        throw new S3Error("InvalidArgument", "the customer key must be 32 bytes of base64", {
            cause,
        });
    }
    if (bytes.length !== CUSTOMER_KEY_BYTES) {
        throw new S3Error("InvalidArgument", "the customer key must be 32 bytes of base64");
    }

    return bytes;
}
