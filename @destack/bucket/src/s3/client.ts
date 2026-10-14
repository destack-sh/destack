import type { Connector } from "@destack/resource";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { copyRequest } from "@destack/runtime/request";
import {
    type Bucket,
    type BucketBody,
    type BucketCondition,
    type BucketFile,
    BucketFileBody,
    type BucketGetOptions,
    BucketHttpMetadata,
    BucketKey,
    BucketListing,
    type BucketListOptions,
    type BucketPutOptions,
    BucketRange,
    MAX_BATCH_FILES,
    type MultipartOptions,
    type MultipartUpload,
    UploadedPart,
    type UploadPartOptions,
} from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import { S3Error } from "./error.ts";
import {
    CHECKSUM_HEADERS,
    CHECKSUM_MODE_HEADER,
    CUSTOMER_KEY_HEADERS,
    customerKeyHeaders,
    S3_STORAGE_CLASSES,
    S3Headers,
} from "./headers.ts";
import { encodeUri } from "./uri.ts";
import { writeXml, XmlNode } from "./xml.ts";

/** The provider code of buckets a workload calls over S3 at its host's egress. */
export const S3_PROVIDER = "s3";

/** The headers a write answers with, which describe the stored file beside what the client sent. */
const ANSWERED_HEADERS = ["etag", "x-amz-version-id", "last-modified"] as const;

/** A refusal a host answers instead of an S3 error document. */
const Refusal = schema
    .object({ code: schema.string(), status: schema.number().int(), message: schema.string() })
    .strip();

/** One S3 request on the bucket. */
interface S3Call {
    /** The HTTP method. */
    readonly method: "GET" | "HEAD" | "PUT" | "POST" | "DELETE";
    /** The file key, absent for a request on the bucket. */
    readonly key?: string;
    /** The query parameters. */
    readonly query?: Readonly<Record<string, string | undefined>>;
    /** The request headers. */
    readonly headers?: Headers;
    /** The request body. */
    readonly body?:
        | Uint8Array<ArrayBuffer>
        | Blob
        | ArrayBuffer
        | ReadableStream<Uint8Array>
        | string;
}

/** A bucket reached over the path-style S3 API at its URL, as S3Server serves it. */
export class S3Client implements Bucket {
    /** The connector of buckets a host serves over S3 at its egress. */
    static readonly connector: Connector<Bucket> = {
        code: S3_PROVIDER,
        connect: async (binding) => {
            // require the credential the egress identifies the instance by
            const credential = binding.credential;
            if (credential === undefined) {
                throw new TypeError(`bucket ${binding.resource} has no credential`);
            }

            // send every request with the credential
            const send = (request: Request) => {
                // read bodies as stored, never decoded by their stored content encoding
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${credential}`);
                const stored: RequestInit & { readonly decompress: false } = { decompress: false };

                return fetch(copyRequest(request, { headers }), stored);
            };

            return Object.assign(new S3Client(binding.reference, send), {
                [Symbol.asyncDispose]: () => Promise.resolve(),
            });
        },
    };

    /** The bucket's URL, below which keys are addressed path-style. */
    readonly url: string;
    /** Send one request to the bucket. */
    readonly #fetch: (request: Request) => Promise<Response>;

    /** Reach a bucket at its URL through a fetch. */
    constructor(url: string, fetch: (request: Request) => Promise<Response>) {
        this.url = url.replace(/\/$/u, "");
        this.#fetch = fetch;
    }

    /** Read metadata; return null when the key does not exist. */
    async head(key: string): Promise<BucketFile | null> {
        // read the file's headers with its checksums
        BucketKey.check(key);
        const headers = new Headers({ [CHECKSUM_MODE_HEADER]: "ENABLED" });
        const answer = await this.#send({ method: "HEAD", key, headers });

        // answer nothing for a missing file, and refuse any other failure by its status
        if (answer.status === 404) {
            return null;
        } else if (!answer.ok) {
            throw S3Error.answered(answer.status, `reading ${key} answered ${answer.status}`);
        }

        return S3Headers.read(key, sizeOf(answer.headers), answer.headers);
    }

    /** Read a file, null when the key does not exist. */
    async get(key: string, options: BucketGetOptions = {}): Promise<BucketFileBody | null> {
        // ask for the range, under the conditions and customer key, with the file's checksums
        BucketKey.check(key);
        const headers = new Headers({ [CHECKSUM_MODE_HEADER]: "ENABLED" });
        if (options.range !== undefined) {
            BucketRange.check(options.range);
            headers.set("range", rangeHeader(options.range));
        }
        writeCondition(headers, options.onlyIf);
        writeCustomerKey(headers, options.ssecKey);
        const answer = await this.#send({ method: "GET", key, headers });

        // refuse a failed precondition with the current file, and answer nothing for a missing one
        if (answer.status === 304 || answer.status === 412) {
            throw BucketError.preconditionFailed(await this.head(key));
        } else if (!answer.ok) {
            const failure = await refusal(answer);
            if (failure instanceof BucketError && failure.code === "NO_SUCH_KEY") {
                return null;
            }
            throw failure;
        }

        // keep the body with the file and the range it returns
        const file = S3Headers.read(key, sizeOf(answer.headers), answer.headers);
        const body = answer.body ?? new ReadableStream({ start: (empty) => empty.close() });

        return new BucketFileBody(file, body, rangeOf(answer.headers));
    }

    /** Store a complete file, atomically replacing any previous one. */
    async put(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions = {},
    ): Promise<BucketFile> {
        // send the stored metadata, preconditions and expected digests with the body
        BucketKey.check(key);
        const headers = writtenHeaders(options);
        writeCondition(headers, options.onlyIf);
        if (options.md5 !== undefined) {
            headers.set("content-md5", base64(options.md5));
        }
        for (const [algorithm, name] of CHECKSUM_HEADERS) {
            const value = options[algorithm];
            if (value !== undefined) {
                headers.set(name, base64(value));
            }
        }
        const sent = counted(body);
        const answer = await this.#send({ method: "PUT", key, headers, body: sent.body });

        // refuse a failed precondition with the current file, absent for a missing If-Match target
        if (answer.status === 412) {
            throw BucketError.preconditionFailed(await this.head(key));
        } else if (!answer.ok) {
            const failure = await refusal(answer);
            const isMissing = failure instanceof BucketError && failure.code === "NO_SUCH_KEY";
            throw isMissing && options.onlyIf?.etagMatches !== undefined
                ? BucketError.preconditionFailed(null)
                : failure;
        }

        return written(key, sent.size(), headers, answer.headers);
    }

    /** Delete up to 1000 files; absent keys remain absent. */
    async delete(key: string | string[]): Promise<void> {
        // delete one file by its key
        if (typeof key === "string") {
            BucketKey.check(key);
            await this.#expect(await this.#send({ method: "DELETE", key }));

            return;
        }

        // delete several files with one document, checking every key first
        if (key.length > MAX_BATCH_FILES) {
            throw new BucketError("INVALID_LIMIT", "a delete accepts at most 1000 files");
        }
        for (const each of key) {
            BucketKey.check(each);
        }
        if (key.length === 0) {
            return;
        }
        const document = writeXml("Delete", {
            Quiet: true,
            Object: key.map((each) => ({ Key: each })),
        });
        const headers = new Headers({ "x-amz-checksum-sha256": await sha256(document) });
        const answer = await this.#send({
            method: "POST",
            query: { delete: "" },
            headers,
            body: document,
        });

        // refuse the first key the bucket refused
        const [failed] = (await this.#document(answer)).all("Error");
        if (failed !== undefined) {
            throw readFailure(failed);
        }
    }

    /** List a page of entries. */
    async list(options: BucketListOptions = {}): Promise<BucketListing> {
        // check the prefix and page size as the hosts do
        const prefix = options.prefix ?? "";
        if (prefix !== "") {
            BucketKey.check(prefix);
        }
        const limit = options.limit ?? MAX_BATCH_FILES;
        BucketListing.checkLimit(limit);

        // list one page with version 2 of the listing API
        const answer = await this.#send({
            method: "GET",
            query: {
                "list-type": "2",
                prefix: options.prefix,
                delimiter: options.delimiter,
                "continuation-token": options.cursor,
                "start-after": options.startAfter,
                "max-keys": String(limit),
            },
        });
        const document = await this.#document(answer);

        // read the entries, the grouped prefixes and the continuation
        const files = document.all("Contents").map((entry) => ({
            key: required(entry, "Key"),
            size: Number(required(entry, "Size")),
            etag: required(entry, "ETag").replace(/^"(.*)"$/u, "$1"),
            uploaded: new Date(required(entry, "LastModified")),
            storageClass: S3Headers.storageClassNamed(required(entry, "StorageClass")),
        }));
        const delimitedPrefixes = document
            .all("CommonPrefixes")
            .map((group) => required(group, "Prefix"));
        const cursor = document.value("NextContinuationToken");

        return document.value("IsTruncated") === "true" && cursor !== undefined
            ? { files, delimitedPrefixes, truncated: true, cursor }
            : { files, delimitedPrefixes, truncated: false };
    }

    /** Create an incomplete file upload. */
    async createMultipartUpload(
        key: string,
        options: MultipartOptions = {},
    ): Promise<MultipartUpload> {
        // start the upload with the metadata its file receives
        BucketKey.check(key);
        const headers = writtenHeaders(options);
        const answer = await this.#send({ method: "POST", key, query: { uploads: "" }, headers });
        const document = await this.#document(answer);

        return this.resumeMultipartUpload(key, required(document, "UploadId"));
    }

    /** Reference an existing upload; subsequent operations check its existence. */
    resumeMultipartUpload(key: string, uploadId: string): MultipartUpload {
        return {
            key,
            uploadId,
            uploadPart: (partNumber, body, options) =>
                this.#uploadPart(key, uploadId, partNumber, body, options),
            complete: (uploaded) => this.#complete(key, uploadId, uploaded),
            abort: async () => {
                const query = { uploadId };
                await this.#expect(await this.#send({ method: "DELETE", key, query }));
            },
        };
    }

    /** Store one numbered part of an upload. */
    async #uploadPart(
        key: string,
        uploadId: string,
        partNumber: number,
        body: BucketBody,
        options: UploadPartOptions = {},
    ): Promise<UploadedPart> {
        // send the part under the upload's customer key
        UploadedPart.checkNumber(partNumber);
        const headers = new Headers();
        writeCustomerKey(headers, options.ssecKey);
        const query = { partNumber: String(partNumber), uploadId };
        const sent = counted(body);
        const answer = await this.#send({ method: "PUT", key, query, headers, body: sent.body });

        // read the part's entity tag
        const etag = (await this.#expect(answer)).headers.get("etag");
        if (etag === null) {
            throw new TypeError(`the part ${partNumber} of ${uploadId} returned no entity tag`);
        }

        return { partNumber, etag: etag.replace(/^"(.*)"$/u, "$1") };
    }

    /** Assemble the selected parts of an upload into one file. */
    async #complete(key: string, uploadId: string, uploaded: UploadedPart[]): Promise<BucketFile> {
        // assemble the selected parts
        const document = writeXml("CompleteMultipartUpload", {
            Part: uploaded.map((part) => ({ PartNumber: part.partNumber, ETag: `"${part.etag}"` })),
        });
        const query = { uploadId };
        const answer = await this.#send({ method: "POST", key, query, body: document });
        const version = (await this.#expect(answer)).headers.get("x-amz-version-id");

        // read the assembled file, whose metadata the upload's creation set, refusing a later version
        const file = await this.head(key);
        if (file === null || file.version !== version) {
            throw BucketError.preconditionFailed(file);
        }

        return file;
    }

    /** Send one request to the bucket or one of its keys. */
    #send(call: S3Call): Promise<Response> {
        // address the key path-style below the bucket, with the query encoded as S3 signs it
        const path = call.key === undefined ? "" : `/${encodeUri(call.key, true)}`;
        const query = Object.entries(call.query ?? {}).flatMap(([name, value]) =>
            value === undefined ? [] : [`${encodeUri(name, false)}=${encodeUri(value, false)}`],
        );
        const url = `${this.url}${path}${query.length === 0 ? "" : `?${query.join("&")}`}`;

        // stream a body of unknown length once
        const request = new Request(url, {
            method: call.method,
            headers: call.headers ?? new Headers(),
            body: call.body ?? null,
            ...(call.body instanceof ReadableStream ? { duplex: "half" } : {}),
        });

        return this.#fetch(request);
    }

    /** Require a successful answer, refusing as the bucket or its host refused. */
    async #expect(answer: Response): Promise<Response> {
        if (!answer.ok) {
            throw await refusal(answer);
        }

        return answer;
    }

    /** Read the XML document of a successful answer. */
    async #document(answer: Response): Promise<XmlNode> {
        return XmlNode.parse(await (await this.#expect(answer)).text());
    }
}

/** Read an answer's refusal. */
async function refusal(answer: Response): Promise<Error> {
    // read an S3 error document
    const text = await answer.text();
    if (answer.headers.get("content-type") === "application/xml") {
        return readFailure(XmlNode.parse(text));
    }

    // read the refusal of the host in front of the bucket
    const refused = Refusal.parse(JSON.parse(text));

    return new CallError(refused.code, { status: refused.status, message: refused.message });
}

/** Read the storage failure of an S3 error element, else the S3 error itself. */
function readFailure(element: XmlNode): Error {
    const failure = S3Error.named(required(element, "Code"), required(element, "Message"));

    return failure.toBucketError() ?? failure;
}

/** Read the text of a child an S3 document requires. */
function required(element: XmlNode, name: string): string {
    const value = element.value(name);
    if (value === undefined) {
        throw new TypeError(`the S3 element ${element.name} has no ${name}`);
    }

    return value;
}

/** Read a file's whole size: the total of a returned range, else the body's length. */
function sizeOf(headers: Headers): number {
    // take the total of a range
    const range = headers.get("content-range");
    if (range !== null) {
        return Number(range.slice(range.lastIndexOf("/") + 1));
    }

    // take the length of the whole body
    const length = headers.get("content-length");
    if (length === null) {
        throw new TypeError("the answer has no content length");
    }

    return Number(length);
}

/** Read the returned range of a partial answer, absent for a whole body. */
function rangeOf(headers: Headers): { offset: number; length: number } | undefined {
    // read the first and last byte of a partial answer
    const match = /^bytes (\d+)-(\d+)\/\d+$/u.exec(headers.get("content-range") ?? "");
    if (match === null) {
        return undefined;
    }
    const offset = Number(match[1]);

    return { offset, length: Number(match[2]) - offset + 1 };
}

/** Write a range as a Range header. */
function rangeHeader(range: BucketRange): string {
    // select the trailing bytes
    if ("suffix" in range) {
        return `bytes=-${range.suffix}`;
    }

    // select from the offset to its length, else to the end
    const offset = range.offset ?? 0;

    return range.length === undefined
        ? `bytes=${offset}-`
        : `bytes=${offset}-${offset + range.length - 1}`;
}

/** Write preconditions as conditional headers, which compare upload times in whole seconds. */
function writeCondition(headers: Headers, condition: BucketCondition | undefined): void {
    // refuse a time HTTP dates cannot express, unless the caller compares whole seconds
    if (condition === undefined) {
        return;
    }
    const times = [condition.uploadedBefore, condition.uploadedAfter];
    const isWhole = times.every((time) => time === undefined || time.getTime() % 1000 === 0);
    if (!isWhole && condition.secondsGranularity !== true) {
        throw new BucketError("UNSUPPORTED", "conditions over S3 compare only whole seconds");
    }

    // write each condition the caller sets
    if (condition.etagMatches !== undefined) {
        headers.set("if-match", entityTag(condition.etagMatches));
    }
    if (condition.etagDoesNotMatch !== undefined) {
        headers.set("if-none-match", entityTag(condition.etagDoesNotMatch));
    }
    if (condition.uploadedBefore !== undefined) {
        headers.set("if-unmodified-since", condition.uploadedBefore.toUTCString());
    }
    if (condition.uploadedAfter !== undefined) {
        headers.set("if-modified-since", condition.uploadedAfter.toUTCString());
    }
}

/** Quote an entity tag, keeping the asterisk. */
function entityTag(etag: string): string {
    return etag === "*" ? "*" : `"${etag}"`;
}

/** Write the headers of a file's stored metadata, storage class, lock and customer key. */
function writtenHeaders(options: MultipartOptions): Headers {
    // write the HTTP metadata and user metadata
    const headers = new Headers();
    BucketHttpMetadata.write(options.httpMetadata ?? {}, headers);
    for (const [name, value] of Object.entries(options.customMetadata ?? {})) {
        headers.set(`x-amz-meta-${name}`, value);
    }

    // write the storage class, the compliance lock and the customer key
    if (options.storageClass !== undefined) {
        headers.set("x-amz-storage-class", S3_STORAGE_CLASSES[options.storageClass]);
    }
    if (options.retainUntil !== undefined) {
        headers.set("x-amz-object-lock-mode", "COMPLIANCE");
        headers.set("x-amz-object-lock-retain-until-date", options.retainUntil.toISOString());
    }
    writeCustomerKey(headers, options.ssecKey);

    return headers;
}

/** Write the SSE-C headers of a customer key given as raw bytes or hexadecimal. */
function writeCustomerKey(headers: Headers, key: ArrayBuffer | string | undefined): void {
    // pass over a request without a key
    if (key === undefined) {
        return;
    }

    // send the key as base64 with its digest
    const bytes = typeof key === "string" ? Uint8Array.fromHex(key) : new Uint8Array(key);
    for (const [name, value] of Object.entries(customerKeyHeaders(bytes.toBase64()))) {
        headers.set(name, value);
    }
}

/** Describe a written file from what the client sent. */
function written(key: string, size: number, sent: Headers, answered: Headers): BucketFile {
    // keep the sent metadata without the customer key and the expected digests
    const headers = new Headers(sent);
    headers.delete(CUSTOMER_KEY_HEADERS.key);
    headers.delete("content-md5");
    for (const [, name] of CHECKSUM_HEADERS) {
        headers.delete(name);
    }

    // add the entity tag, version, upload time and stored checksums the bucket answered
    for (const name of [...ANSWERED_HEADERS, ...CHECKSUM_HEADERS.map(([, header]) => header)]) {
        const value = answered.get(name);
        if (value !== null) {
            headers.set(name, value);
        }
    }

    return S3Headers.read(key, size, headers);
}

/** Send a body as fetch takes it, counting the bytes a stream sends. */
function counted(body: BucketBody | null): {
    readonly body: Exclude<S3Call["body"], undefined>;
    size(): number;
} {
    // count a stream's bytes as they pass
    if (body instanceof ReadableStream) {
        let size = 0;
        const counting = new TransformStream<Uint8Array, Uint8Array>({
            transform: (chunk, controller) => {
                size += chunk.byteLength;
                controller.enqueue(chunk);
            },
        });

        return { body: body.pipeThrough(counting), size: () => size };
    }

    // measure a body of known length
    const bytes =
        body === null
            ? new Uint8Array()
            : typeof body === "string"
              ? new TextEncoder().encode(body)
              : ArrayBuffer.isView(body)
                ? new Uint8Array(body.buffer, body.byteOffset, body.byteLength)
                : body;

    return { body: bytes, size: () => (bytes instanceof Blob ? bytes.size : bytes.byteLength) };
}

/** Encode a digest given as hexadecimal text or raw bytes as base64. */
function base64(value: string | ArrayBuffer | ArrayBufferView): string {
    // decode hexadecimal text
    if (typeof value === "string") {
        return Uint8Array.fromHex(value).toBase64();
    }

    // encode the bytes
    const bytes = ArrayBuffer.isView(value)
        ? new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
        : new Uint8Array(value);

    return bytes.toBase64();
}

/** Compute the base64 SHA-256 checksum of a request document. */
async function sha256(document: string): Promise<string> {
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(document));

    return new Uint8Array(digest).toBase64();
}
