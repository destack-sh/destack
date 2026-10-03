import type { BucketFile } from "../bucket/index.ts";
import { BucketFileBody, BucketRange, MAX_BATCH_FILES } from "../bucket/index.ts";
import { BucketKey } from "../bucket/key.ts";
import type { BucketCopyOptions, S3Bucket } from "./bucket.ts";
import { S3Condition } from "./condition.ts";
import { S3Error } from "./error.ts";
import { type CopySource, keyEncoder, xmlResponse } from "./operation.ts";
import { S3_STORAGE_CLASSES, type S3Request } from "./request.ts";
import type { XmlElement } from "./xml.ts";

/** The response headers GetObject overrides from query parameters. */
const RESPONSE_OVERRIDES = {
    "response-cache-control": "cache-control",
    "response-content-disposition": "content-disposition",
    "response-content-encoding": "content-encoding",
    "response-content-language": "content-language",
    "response-content-type": "content-type",
    "response-expires": "expires",
} as const;
/** The query parameters of ListObjectsV2. */
const LIST_PARAMETERS = [
    "list-type",
    "prefix",
    "delimiter",
    "max-keys",
    "continuation-token",
    "start-after",
    "fetch-owner",
    "encoding-type",
];

/** The S3 operations on objects and their listing. */
export const S3Object = {
    get: getObject,
    head: headObject,
    put: putObject,
    delete: deleteObject,
    copy: copyObject,
    list: listObjects,
    deleteMany: deleteObjects,
};

/** Answer GetObject with the file's bytes, a range of them, or a failed precondition. */
async function getObject(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // resolve the request's conditions, answering from the current file when they already fail
    call.checkParameters(Object.keys(RESPONSE_OVERRIDES));
    const condition = call.condition();
    const resolution = await S3Condition.resolve(condition, () => bucket.head(call.key));
    if ("failed" in resolution) {
        return preconditionFailure(requireFile(resolution.failed), condition);
    }

    // read the file under the resolved conditions and the request's range
    const range = call.range();
    const ssecKey = call.ssecKey();
    const file = requireFile(
        await bucket.get(call.key, {
            ...(resolution.onlyIf === undefined ? {} : { onlyIf: resolution.onlyIf }),
            ...(range === undefined ? {} : { range }),
            ...(ssecKey === undefined ? {} : { ssecKey }),
        }),
    );
    if (!(file instanceof BucketFileBody)) {
        return preconditionFailure(file, condition);
    }

    // describe the returned bytes, with the overrides the query names
    const headers = fileHeaders(file);
    headers.set("content-length", String(file.range?.length ?? file.size));
    if (file.range !== undefined) {
        const last = file.range.offset + file.range.length - 1;
        headers.set("content-range", `bytes ${file.range.offset}-${last}/${file.size}`);
    }
    for (const [parameter, header] of Object.entries(RESPONSE_OVERRIDES)) {
        const value = call.query.get(parameter);
        if (value !== undefined) {
            headers.set(header, value);
        }
    }

    return new Response(file.body, { status: file.range === undefined ? 200 : 206, headers });
}

/** Answer HeadObject with the file's metadata, or a failed precondition. */
async function headObject(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // read the metadata and evaluate the conditions against it
    call.checkParameters(Object.keys(RESPONSE_OVERRIDES));
    const file = requireFile(await bucket.head(call.key));
    const condition = call.condition();
    if (!S3Condition.matches(file, condition)) {
        return preconditionFailure(file, condition);
    }

    // describe the whole file, or the requested range of it
    const headers = fileHeaders(file);
    const range = call.range();
    headers.set("content-length", String(file.size));
    if (range === undefined) {
        return new Response(null, { status: 200, headers });
    }
    const { offset, length } = BucketRange.resolve(file.size, range);
    headers.set("content-length", String(length));
    headers.set("content-range", `bytes ${offset}-${offset + length - 1}/${file.size}`);

    return new Response(null, { status: 206, headers });
}

/** Answer PutObject by streaming the body into the bucket. */
async function putObject(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // accept the conditional writes S3 accepts: If-Match, and If-None-Match with an asterisk
    call.checkParameters([]);
    const condition = call.condition();
    if (condition?.etagDoesNotMatch !== undefined && condition.etagDoesNotMatch !== "*") {
        throw new S3Error("NotImplemented", "conditional writes support only If-None-Match: *");
    }
    const resolution = await S3Condition.resolve(condition, () => bucket.head(call.key));
    if ("failed" in resolution) {
        return writeConditionFailure(call, resolution.failed);
    }

    // store the verified body with the request's metadata
    const storageClass = call.storageClass();
    const ssecKey = call.ssecKey();
    const file = await bucket.put(call.key, call.body(), {
        httpMetadata: call.httpMetadata(),
        customMetadata: call.customMetadata(),
        ...(storageClass === undefined ? {} : { storageClass }),
        ...(ssecKey === undefined ? {} : { ssecKey }),
        ...(resolution.onlyIf === undefined ? {} : { onlyIf: resolution.onlyIf }),
    });
    if (file === null) {
        return writeConditionFailure(call, await bucket.head(call.key));
    }

    return new Response(null, { headers: writeHeaders(call, file) });
}

/** Answer DeleteObject, which succeeds whether or not the key exists. */
async function deleteObject(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // refuse parameters and conditions, then delete
    call.checkParameters([]);
    if (call.condition() !== undefined) {
        throw new S3Error("NotImplemented", "conditional deletes are not supported");
    }
    await bucket.delete(call.key);

    return new Response(null, { status: 204 });
}

/** Answer CopyObject, sharing contents within a bucket and streaming them between buckets. */
async function copyObject(
    call: S3Request,
    bucket: S3Bucket,
    source: CopySource,
): Promise<Response> {
    // read the metadata directive and refuse copies that change nothing
    call.checkParameters([]);
    const directive = call.headers.get("x-amz-metadata-directive") ?? "COPY";
    if (directive !== "COPY" && directive !== "REPLACE") {
        throw new S3Error("InvalidArgument", "x-amz-metadata-directive must be COPY or REPLACE");
    }
    const storageClass = call.storageClass();
    if (
        source.isSameBucket &&
        source.key === call.key &&
        directive === "COPY" &&
        storageClass === undefined
    ) {
        throw new S3Error(
            "InvalidRequest",
            "this copy request is illegal because it copies an object to itself without changing its metadata or storage class",
        );
    }
    if (call.ssecKey() !== undefined) {
        throw new S3Error("NotImplemented", "customer keys are not supported on copies");
    }

    // copy under the source's conditions, with replaced metadata when asked
    const metadata =
        directive === "REPLACE"
            ? { httpMetadata: call.httpMetadata(), customMetadata: call.customMetadata() }
            : {};
    const options = {
        ...metadata,
        ...(storageClass === undefined ? {} : { storageClass }),
        ...(source.onlyIf === undefined ? {} : { onlyIf: source.onlyIf }),
    };
    const file = source.isSameBucket
        ? await bucket.copy(source.key, call.key, options)
        : await copyBetween(source, bucket, call.key, options);
    if (file === null) {
        throw new S3Error(
            "PreconditionFailed",
            "at least one of the preconditions you specified did not hold",
        );
    }

    return xmlResponse("CopyObjectResult", { LastModified: file.uploaded, ETag: file.httpEtag });
}

/** Answer ListObjectsV2 with a page of files and common prefixes. */
async function listObjects(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // read the selection and page size, with at most 1000 keys per page as S3 returns
    call.checkParameters(LIST_PARAMETERS);
    const encode = keyEncoder(call);
    const prefix = call.query.get("prefix") ?? "";
    const delimiter = call.query.get("delimiter") ?? "";
    const token = call.query.get("continuation-token");
    const startAfter = call.query.get("start-after");
    const limit = Math.min(
        call.integer("max-keys", 0, Number.MAX_SAFE_INTEGER) ?? MAX_BATCH_FILES,
        MAX_BATCH_FILES,
    );

    // list one page, or none for a zero page size
    const page =
        limit === 0
            ? { files: [], delimitedPrefixes: [], truncated: false as const }
            : await bucket.list({
                  ...(prefix === "" ? {} : { prefix }),
                  ...(delimiter === "" ? {} : { delimiter }),
                  ...(token === undefined ? {} : { cursor: token }),
                  ...(startAfter === undefined ? {} : { startAfter }),
                  limit,
              });

    return xmlResponse("ListBucketResult", {
        Name: call.bucketName,
        Prefix: encode(prefix),
        Delimiter: delimiter === "" ? undefined : encode(delimiter),
        MaxKeys: limit,
        EncodingType: call.query.get("encoding-type"),
        KeyCount: page.files.length + page.delimitedPrefixes.length,
        IsTruncated: page.truncated,
        ContinuationToken: token,
        NextContinuationToken: page.truncated ? page.cursor : undefined,
        StartAfter: startAfter === undefined ? undefined : encode(startAfter),
        Contents: page.files.map((file) => ({
            Key: encode(file.key),
            LastModified: file.uploaded,
            ETag: file.httpEtag,
            Size: file.size,
            StorageClass: S3_STORAGE_CLASSES[file.storageClass],
        })),
        CommonPrefixes: page.delimitedPrefixes.map((delimited) => ({
            Prefix: encode(delimited),
        })),
    });
}

/** Answer DeleteObjects, reporting keys S3 would refuse individually. */
async function deleteObjects(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // require an integrity checksum of the document, as S3 does
    call.checkParameters(["delete"]);
    const hasChecksum = [...call.headers.keys()].some(
        (name) =>
            name === "content-md5" ||
            name.startsWith("x-amz-checksum-") ||
            name === "x-amz-trailer",
    );
    if (!hasChecksum) {
        throw new S3Error(
            "InvalidRequest",
            "missing required header for this request: content-md5",
        );
    }

    // read at most 1000 objects
    const document = await call.document();
    const objects = document.all("Object");
    if (document.name !== "Delete" || objects.length === 0 || objects.length > MAX_BATCH_FILES) {
        throw new S3Error(
            "MalformedXML",
            "the document must name 1 through 1000 objects to delete",
        );
    }
    const isQuiet = document.value("Quiet") === "true";

    // refuse versions and invalid keys one by one, and delete the rest together
    const deleted: string[] = [];
    const errors: XmlElement[] = [];
    for (const object of objects) {
        const key = object.value("Key") ?? "";
        const failure = keyFailure(key, object.value("VersionId"));
        if (failure === undefined) {
            deleted.push(key);
        } else {
            errors.push({ Key: key, Code: failure.code, Message: failure.message });
        }
    }
    if (deleted.length > 0) {
        await bucket.delete(deleted);
    }

    return xmlResponse("DeleteResult", {
        Deleted: isQuiet ? [] : deleted.map((key) => ({ Key: key })),
        Error: errors,
    });
}

/** Copy a file from another bucket by streaming it, returning null when a source precondition fails. */
async function copyBetween(
    source: CopySource,
    bucket: S3Bucket,
    key: string,
    options: BucketCopyOptions,
): Promise<BucketFile | null> {
    // read the source under its conditions
    const file = requireFile(
        await source.bucket.get(
            source.key,
            options.onlyIf === undefined ? {} : { onlyIf: options.onlyIf },
        ),
    );
    if (!(file instanceof BucketFileBody)) {
        return null;
    }

    // write it with its metadata unless the copy replaces it
    return await bucket.put(key, file.body, {
        httpMetadata: options.httpMetadata ?? file.httpMetadata,
        customMetadata: options.customMetadata ?? file.customMetadata,
        ...(options.storageClass === undefined ? {} : { storageClass: options.storageClass }),
    });
}

/** Require an existing file. */
function requireFile<File extends BucketFile>(file: File | null): File {
    if (file === null) {
        throw new S3Error("NoSuchKey", "the specified key does not exist");
    }

    return file;
}

/** Answer a failed read precondition: 412 for If-Match and If-Unmodified-Since, else 304. */
function preconditionFailure(file: BucketFile, condition: S3Condition | undefined): Response {
    // require the condition that failed
    if (condition === undefined) {
        throw new TypeError("a failed precondition needs its condition");
    }

    // refuse a failed requirement, and answer an unmodified file otherwise
    const required = {
        ...(condition.etagMatches === undefined ? {} : { etagMatches: condition.etagMatches }),
        ...(condition.uploadedBefore === undefined
            ? {}
            : { uploadedBefore: condition.uploadedBefore }),
    };
    if (!S3Condition.matches(file, required)) {
        throw new S3Error(
            "PreconditionFailed",
            "at least one of the preconditions you specified did not hold",
        );
    }

    return new Response(null, {
        status: 304,
        headers: { etag: file.httpEtag, "last-modified": file.uploaded.toUTCString() },
    });
}

/** Refuse a failed write precondition: 404 when If-Match names an absent key, else 412. */
function writeConditionFailure(call: S3Request, current: BucketFile | null): never {
    if (call.headers.has("if-match") && current === null) {
        throw new S3Error("NoSuchKey", "the specified key does not exist");
    }
    throw new S3Error(
        "PreconditionFailed",
        "at least one of the preconditions you specified did not hold",
    );
}

/** Describe a file in response headers. */
function fileHeaders(file: BucketFile): Headers {
    // describe the version and its stored HTTP metadata
    const headers = new Headers({
        etag: file.httpEtag,
        "last-modified": file.uploaded.toUTCString(),
        "accept-ranges": "bytes",
    });
    file.writeHttpMetadata(headers);

    // add user metadata, a non-standard storage class and the customer key digest
    for (const [name, value] of Object.entries(file.customMetadata)) {
        headers.set(`x-amz-meta-${name}`, value);
    }
    if (file.storageClass !== "Standard") {
        headers.set("x-amz-storage-class", S3_STORAGE_CLASSES[file.storageClass]);
    }
    if (file.ssecKeyMd5 !== undefined) {
        headers.set("x-amz-server-side-encryption-customer-algorithm", "AES256");
        headers.set("x-amz-server-side-encryption-customer-key-md5", file.ssecKeyMd5);
    }

    return headers;
}

/** Describe a stored file in a write response. */
function writeHeaders(call: S3Request, file: BucketFile): Headers {
    // answer the stored version, and echo the digest of a customer key
    const headers = new Headers({ etag: file.httpEtag });
    const md5 = call.headers.get("x-amz-server-side-encryption-customer-key-md5");
    if (md5 !== null) {
        headers.set("x-amz-server-side-encryption-customer-algorithm", "AES256");
        headers.set("x-amz-server-side-encryption-customer-key-md5", md5);
    }

    return headers;
}

/** Find why S3 would refuse to delete one key, or return undefined. */
function keyFailure(key: string, versionId: string | undefined): S3Error | undefined {
    // refuse versions, which are out of scope
    if (versionId !== undefined) {
        return new S3Error("NotImplemented", "deleting versions is not supported");
    }

    // refuse keys the bucket would refuse
    try {
        BucketKey.check(key);

        return undefined;
    } catch (error) {
        return S3Error.read(error);
    }
}
