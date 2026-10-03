import { aligned } from "@destack/schema";
import { BucketFileBody, MAX_BATCH_FILES, MAX_PART_NUMBER, UploadedPart } from "../bucket/index.ts";
import type { S3Bucket } from "./bucket.ts";
import { S3Error } from "./error.ts";
import { type CopySource, keyEncoder, xmlResponse } from "./operation.ts";
import { S3_STORAGE_CLASSES, type S3Request } from "./request.ts";

/** The query parameters of ListMultipartUploads. */
const LIST_PARAMETERS = [
    "uploads",
    "prefix",
    "key-marker",
    "upload-id-marker",
    "max-uploads",
    "encoding-type",
];

/** The S3 operations on multipart uploads. */
export const S3Upload = {
    create: createUpload,
    part: uploadPart,
    partCopy: uploadPartCopy,
    complete: completeUpload,
    abort: abortUpload,
    listParts,
    list: listUploads,
};

/** Answer CreateMultipartUpload with the new upload's identifier. */
async function createUpload(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // start the upload with the request's metadata, storage class and key
    call.checkParameters(["uploads"]);
    const storageClass = call.storageClass();
    const ssecKey = call.ssecKey();
    const upload = await bucket.createMultipartUpload(call.key, {
        httpMetadata: call.httpMetadata(),
        customMetadata: call.customMetadata(),
        ...(storageClass === undefined ? {} : { storageClass }),
        ...(ssecKey === undefined ? {} : { ssecKey }),
    });

    return xmlResponse("InitiateMultipartUploadResult", {
        Bucket: call.bucketName,
        Key: call.key,
        UploadId: upload.uploadId,
    });
}

/** Answer UploadPart by streaming the body into one part. */
async function uploadPart(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // stream the verified body into the numbered part
    call.checkParameters(["partNumber", "uploadId"]);
    const upload = bucket.resumeMultipartUpload(call.key, call.uploadId());
    const ssecKey = call.ssecKey();
    const part = await upload.uploadPart(
        call.partNumber(),
        call.body(),
        ssecKey === undefined ? {} : { ssecKey },
    );

    return new Response(null, { headers: { etag: `"${part.etag}"` } });
}

/** Answer UploadPartCopy with a part copied from a file or a range of it. */
async function uploadPartCopy(
    call: S3Request,
    bucket: S3Bucket,
    source: CopySource,
): Promise<Response> {
    // read the part, source range and source conditions
    call.checkParameters(["partNumber", "uploadId"]);
    const upload = bucket.resumeMultipartUpload(call.key, call.uploadId());
    const partNumber = call.partNumber();
    const range = call.copySourceRange();
    const options = {
        ...(range === undefined ? {} : { range }),
        ...(source.onlyIf === undefined ? {} : { onlyIf: source.onlyIf }),
    };

    // copy within the bucket, or stream the source from another bucket
    let part;
    if (source.isSameBucket) {
        part = await upload.uploadPartCopy(partNumber, source.key, options);
    } else {
        const file = await source.bucket.get(source.key, options);
        if (file === null) {
            throw new S3Error("NoSuchKey", "the specified key does not exist");
        }
        part =
            file instanceof BucketFileBody ? await upload.uploadPart(partNumber, file.body) : null;
    }
    if (part === null) {
        throw new S3Error(
            "PreconditionFailed",
            "at least one of the preconditions you specified did not hold",
        );
    }

    return xmlResponse("CopyPartResult", { LastModified: part.uploaded, ETag: `"${part.etag}"` });
}

/** Answer CompleteMultipartUpload with the assembled file's entity tag. */
async function completeUpload(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // read the selected parts
    call.checkParameters(["uploadId"]);
    const document = await call.document();
    const parts = document.all("Part").map((part) => ({
        partNumber: Number(part.value("PartNumber")),
        etag: (part.value("ETag") ?? "").replace(/^"(.*)"$/u, "$1"),
    }));
    if (document.name !== "CompleteMultipartUpload" || parts.length === 0) {
        throw new S3Error("MalformedXML", "the document must list the parts to complete");
    }

    // require valid part numbers in ascending order
    for (const part of parts) {
        UploadedPart.checkNumber(part.partNumber);
    }
    if (parts.slice(1).some((part, index) => part.partNumber <= aligned(parts, index).partNumber)) {
        throw new S3Error("InvalidPartOrder", "the list of parts was not in ascending order");
    }

    // assemble the file
    const upload = bucket.resumeMultipartUpload(call.key, call.uploadId());
    const file = await upload.complete(parts);

    return xmlResponse("CompleteMultipartUploadResult", {
        Location: `${call.url.origin}${call.url.pathname}`,
        Bucket: call.bucketName,
        Key: call.key,
        ETag: file.httpEtag,
    });
}

/** Answer AbortMultipartUpload by discarding the upload and its parts. */
async function abortUpload(call: S3Request, bucket: S3Bucket): Promise<Response> {
    call.checkParameters(["uploadId"]);
    await bucket.resumeMultipartUpload(call.key, call.uploadId()).abort();

    return new Response(null, { status: 204 });
}

/** Answer ListParts with a page of an upload's parts. */
async function listParts(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // read one page after the part number marker
    call.checkParameters(["uploadId", "max-parts", "part-number-marker"]);
    const uploadId = call.uploadId();
    const limit = Math.min(
        call.integer("max-parts", 0, Number.MAX_SAFE_INTEGER) ?? MAX_BATCH_FILES,
        MAX_BATCH_FILES,
    );
    const marker = call.integer("part-number-marker", 0, MAX_PART_NUMBER) ?? 0;
    const page =
        limit === 0
            ? { parts: [], truncated: false }
            : await bucket
                  .resumeMultipartUpload(call.key, uploadId)
                  .listParts({ partNumberMarker: marker, limit });

    return xmlResponse("ListPartsResult", {
        Bucket: call.bucketName,
        Key: call.key,
        UploadId: uploadId,
        PartNumberMarker: marker,
        NextPartNumberMarker: page.parts.at(-1)?.partNumber ?? marker,
        MaxParts: limit,
        IsTruncated: page.truncated,
        Part: page.parts.map((part) => ({
            PartNumber: part.partNumber,
            LastModified: part.uploaded,
            ETag: `"${part.etag}"`,
            Size: part.size,
        })),
    });
}

/** Answer ListMultipartUploads with a page of incomplete uploads. */
async function listUploads(call: S3Request, bucket: S3Bucket): Promise<Response> {
    // read one page after the key and upload markers
    call.checkParameters(LIST_PARAMETERS);
    const encode = keyEncoder(call);
    const prefix = call.query.get("prefix");
    const keyMarker = call.query.get("key-marker");
    const uploadIdMarker = call.query.get("upload-id-marker");
    const limit = Math.min(
        call.integer("max-uploads", 1, Number.MAX_SAFE_INTEGER) ?? MAX_BATCH_FILES,
        MAX_BATCH_FILES,
    );
    const page = await bucket.listUploads({
        ...(prefix === undefined ? {} : { prefix }),
        ...(keyMarker === undefined ? {} : { keyMarker }),
        ...(keyMarker === undefined || uploadIdMarker === undefined ? {} : { uploadIdMarker }),
        limit,
    });

    // mark the next page after the last upload of a truncated one
    const last = page.uploads.at(-1);
    if (page.truncated && last === undefined) {
        throw new TypeError("a truncated upload page lists no uploads");
    }
    const next = page.truncated ? last : undefined;

    return xmlResponse("ListMultipartUploadsResult", {
        Bucket: call.bucketName,
        KeyMarker: encode(keyMarker ?? ""),
        UploadIdMarker: uploadIdMarker ?? "",
        NextKeyMarker: next === undefined ? undefined : encode(next.key),
        NextUploadIdMarker: next?.uploadId,
        Prefix: encode(prefix ?? ""),
        EncodingType: call.query.get("encoding-type"),
        MaxUploads: limit,
        IsTruncated: page.truncated,
        Upload: page.uploads.map((upload) => ({
            Key: encode(upload.key),
            UploadId: upload.uploadId,
            StorageClass: S3_STORAGE_CLASSES[upload.storageClass],
            Initiated: upload.initiated,
        })),
    });
}
