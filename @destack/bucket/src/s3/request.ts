import { nth } from "@destack/schema";
import { type BucketRange, UploadedPart } from "../bucket/index.ts";
import { EntityTag, type S3Condition } from "./condition.ts";
import { CryptoHasher } from "./hasher.ts";
import { CUSTOMER_KEY_HEADERS, readCustomerKey } from "./headers.ts";
import { S3Error } from "./error.ts";
import { Payload } from "./payload.ts";
import { PRESIGN_PARAMETERS, type S3Authorization } from "./signature.ts";
import { decodeUri, readQuery } from "./uri.ts";
import { XmlNode } from "./xml.ts";

/** The largest request document in bytes. */
const MAX_DOCUMENT_BYTES = 8 * 1024 * 1024;

/** An authenticated S3 request, addressed path-style to a bucket and optionally a key. */
export class S3Request {
    /** The HTTP request. */
    readonly request: Request;
    /** The request URL. */
    readonly url: URL;
    /** The request headers, with the x-amz headers of a presigned query. */
    readonly headers: Headers;
    /** The decoded query parameters, except those of authentication. */
    readonly query: Map<string, string>;
    /** The addressed bucket name. */
    readonly bucketName: string;
    /** The addressed key, empty for requests to the bucket. */
    readonly key: string;
    /** The verified signature. */
    readonly authorization: S3Authorization;

    /** Read the address and parameters of an authenticated request. */
    constructor(request: Request, bucketName: string, key: string, authorization: S3Authorization) {
        // retain the request and its address
        this.request = request;
        this.url = new URL(request.url);
        this.bucketName = bucketName;
        this.key = key;
        this.authorization = authorization;

        // separate authentication parameters and the x-amz headers of presigned queries
        this.headers = new Headers(request.headers);
        this.query = new Map();
        for (const [name, value] of readQuery(this.url)) {
            const lowercase = name.toLowerCase();
            if (PRESIGN_PARAMETERS.some((parameter) => parameter === name)) {
                continue;
            }
            if (lowercase.startsWith("x-amz-")) {
                this.headers.set(lowercase, value);
            } else {
                this.query.set(name, value);
            }
        }
    }

    /** The path of the addressed bucket and key in error documents. */
    get resource(): string {
        return this.url.pathname;
    }

    /** Refuse query parameters an operation does not support, such as unsupported subresources. */
    checkParameters(allowed: string[]): void {
        for (const name of this.query.keys()) {
            if (name !== "x-id" && !allowed.includes(name)) {
                throw new S3Error("NotImplemented", `the parameter ${name} is not supported`);
            }
        }
    }

    /** Read an integer query parameter within bounds, or undefined when absent. */
    integer(name: string, minimum: number, maximum: number): number | undefined {
        // read the parameter, which may be absent
        const value = this.query.get(name);
        if (value === undefined) {
            return undefined;
        }

        // require a decimal integer within the bounds
        const number = /^\d{1,15}$/u.test(value) ? Number(value) : Number.NaN;
        if (!(number >= minimum && number <= maximum)) {
            throw new S3Error(
                "InvalidArgument",
                `${name} must be an integer from ${minimum} through ${maximum}`,
            );
        }

        return number;
    }

    /** Read the upload of an upload request. */
    uploadId(): string {
        const uploadId = this.query.get("uploadId");
        if (uploadId === undefined) {
            throw new S3Error("InvalidArgument", "upload requests require an uploadId");
        }

        return uploadId;
    }

    /** Read the part number of a part request. */
    partNumber(): number {
        // require the parameter within the part limit
        const partNumber = this.integer("partNumber", 0, Number.MAX_SAFE_INTEGER);
        if (partNumber === undefined) {
            throw new S3Error("InvalidArgument", "part requests require a partNumber");
        }
        UploadedPart.checkNumber(partNumber);

        return partNumber;
    }

    /** Read the conditional headers of the request or its copy source, with entity-tag lists. */
    condition(prefix: "" | "x-amz-copy-source-" = ""): S3Condition | undefined {
        // read the entity tag and date conditions
        const matches = this.headers.get(`${prefix}if-match`);
        const differs = this.headers.get(`${prefix}if-none-match`);
        const before = this.headers.get(`${prefix}if-unmodified-since`);
        const after = this.headers.get(`${prefix}if-modified-since`);
        if (matches === null && differs === null && before === null && after === null) {
            return undefined;
        }

        // ignore malformed HTTP dates, as HTTP requires of conditional requests
        return {
            ...(matches === null ? {} : { etagMatches: EntityTag.readList(matches) }),
            ...(differs === null ? {} : { etagDoesNotMatch: EntityTag.readList(differs) }),
            ...(before === null || !Number.isFinite(Date.parse(before))
                ? {}
                : { uploadedBefore: new Date(before) }),
            ...(after === null || !Number.isFinite(Date.parse(after))
                ? {}
                : { uploadedAfter: new Date(after) }),
        };
    }

    /** Read one byte range of the Range header. */
    range(): BucketRange | undefined {
        // match one span with at least one end, whose last byte does not precede its first
        const match = /^bytes=(\d*)-(\d*)$/u.exec(this.headers.get("range")?.trim() ?? "");
        if (match === null) {
            return undefined;
        }
        const first = nth(match, 1);
        const last = nth(match, 2);
        if (
            (first === "" && last === "") ||
            (first !== "" && last !== "" && Number(last) < Number(first))
        ) {
            return undefined;
        }

        return readRange(first, last);
    }

    /** Read the copy source range, which must have both ends. */
    copySourceRange(): BucketRange | undefined {
        // read the optional header
        const value = this.headers.get("x-amz-copy-source-range");
        if (value === null) {
            return undefined;
        }

        // require both ends in order
        const match = /^bytes=(\d+)-(\d+)$/u.exec(value.trim());
        const first = match === null ? undefined : nth(match, 1);
        const last = match === null ? undefined : nth(match, 2);
        if (first === undefined || last === undefined || Number(last) < Number(first)) {
            throw new S3Error(
                "InvalidArgument",
                "x-amz-copy-source-range must have the form bytes=first-last",
            );
        }

        return readRange(first, last);
    }

    /** Read the bucket and key in the copy source header. */
    copySource(): { bucketName: string; key: string } {
        // refuse versioned sources and customer keys of the source
        const value = this.headers.get("x-amz-copy-source");
        if (value === null) {
            throw new TypeError("a copy has no x-amz-copy-source header");
        }
        const parts = value.split("?");
        if (parts.length > 1) {
            throw new S3Error("NotImplemented", "copy sources with a version are not supported");
        }
        if (this.headers.has("x-amz-copy-source-server-side-encryption-customer-algorithm")) {
            throw new S3Error("NotImplemented", "customer keys of copy sources are not supported");
        }

        // split the bucket from the key after an optional leading slash
        const source = decodeUri(nth(parts, 0).replace(/^\//u, ""));
        const separator = source.indexOf("/");
        if (separator <= 0 || separator === source.length - 1) {
            throw new S3Error("InvalidArgument", "x-amz-copy-source must give a bucket and a key");
        }

        return { bucketName: source.slice(0, separator), key: source.slice(separator + 1) };
    }

    /** Read a customer-provided encryption key as hexadecimal, verifying its MD5 digest. */
    ssecKey(): string | undefined {
        // refuse server-managed encryption, which is out of scope
        if (this.headers.has("x-amz-server-side-encryption")) {
            throw new S3Error(
                "NotImplemented",
                "server-side encryption with managed keys is not supported",
            );
        }

        // accept no key, or all three headers of one
        const algorithm = this.headers.get(CUSTOMER_KEY_HEADERS.algorithm);
        const key = this.headers.get(CUSTOMER_KEY_HEADERS.key);
        const md5 = this.headers.get(CUSTOMER_KEY_HEADERS.md5);
        if (algorithm === null && key === null && md5 === null) {
            return undefined;
        }
        if (algorithm !== "AES256") {
            throw new S3Error(
                "InvalidEncryptionAlgorithmError",
                "customer keys require the AES256 algorithm",
            );
        }

        // require a 256-bit key and its matching digest
        const bytes = readCustomerKey(key ?? "");
        const digest = new CryptoHasher("md5");
        digest.update(bytes);
        if (digest.digest().toBase64() !== md5) {
            throw new S3Error("InvalidArgument", "the customer key MD5 does not match the key");
        }

        return bytes.toHex();
    }

    /** Stream the request body, decoded and verified against its declared hashes. */
    body(): ReadableStream<Uint8Array> {
        return Payload.open(this.request, this.headers, this.authorization);
    }

    /** Read the request body as an XML document. */
    async document(): Promise<XmlNode> {
        // collect the verified body up to the document limit
        const chunks: Uint8Array[] = [];
        let size = 0;
        for await (const chunk of this.body()) {
            size += chunk.length;
            if (size > MAX_DOCUMENT_BYTES) {
                throw new S3Error("MaxMessageLengthExceeded", "the request document is too large");
            }
            chunks.push(chunk);
        }

        // decode UTF-8 strictly before parsing
        const bytes = new Uint8Array(size);
        let offset = 0;
        for (const chunk of chunks) {
            bytes.set(chunk, offset);
            offset += chunk.length;
        }
        try {
            return XmlNode.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
        } catch (cause) {
            if (cause instanceof S3Error) {
                throw cause;
            }
            throw new S3Error("MalformedXML", "the request document is not valid UTF-8", { cause });
        }
    }
}

/** Build a range from its inclusive first and last byte, either of which may be empty. */
function readRange(first: string, last: string): BucketRange {
    // select a suffix
    if (first === "") {
        return { suffix: Number(last) };
    }
    // select from the first byte to the end
    else if (last === "") {
        return { offset: Number(first) };
    }
    // select the inclusive span
    else {
        return { offset: Number(first), length: Number(last) - Number(first) + 1 };
    }
}
