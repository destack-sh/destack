# @destack/bucket

Declare file storage, read and write its files, and serve it over S3.

## Declarations

`defineBucket` declares a bucket by name, and `write: "system"` lets the system alone write its files.

```ts
import { defineBucket } from "@destack/bucket";

export const files = defineBucket({ name: "files", spec: {} });
export const build = defineBucket({ name: "build", spec: { write: "system" } });
```

## Files

`files.get(context)` returns the bound bucket, whose `put` and `get` write and read files by key.

```ts
const bucket = files.get(context);
await bucket.put("documents/readme.txt", "Hello", { httpMetadata: { contentType: "text/plain" } });
const document = await bucket.get("documents/readme.txt");
const text = document ? await document.text() : undefined;
```

## Multipart uploads

`createMultipartUpload` starts an upload in parts, and `complete` joins the uploaded parts into one file.

```ts
const upload = await bucket.createMultipartUpload("media/video.mp4", {
    httpMetadata: { contentType: "video/mp4" },
});
const first = await upload.uploadPart(1, firstChunk);
const last = await upload.uploadPart(2, lastChunk);
await upload.complete([first, last]);
```

## Encryption

`ssecKey` encrypts a file's content with a customer key, and every read of the file passes the same key.

```ts
await bucket.put("private/report.pdf", report, { ssecKey });
const sealed = await bucket.get("private/report.pdf", { ssecKey });
```

## Hosts

`LocalBucket.open` opens a bucket in a local directory, and `context.bind` binds a declared bucket to it.

```ts
import { LocalBucket } from "@destack/bucket/local";
import { ResourceContext } from "@destack/resource/context";

await using bucket = await LocalBucket.open("./resources/files");
const context = new ResourceContext();
context.bind(files, bucket);
```

## Workers

`R2Bucket` wraps an R2 bucket binding of a Workers environment.

```ts
import { R2Bucket } from "@destack/bucket/r2";

context.bind(files, new R2Bucket(environment.FILES));
```

## Device hosts

`LocalBucketHost` keeps a device host's buckets in local directories, and `bucketProvider.local` provides them to the host's spaces.

```ts
import { LocalBucketHost } from "@destack/bucket/local";
import { bucketProvider } from "@destack/bucket/provider";

const credentials = await LocalBucketHost.credentials(keychain, `s3/${hostId}`);
const buckets = new LocalBucketHost({ directory, endpoint, region: "local", credentials });
const provider = bucketProvider.local(buckets);
```

## Cells

`R2BucketHost` keeps a cell's buckets as prefixes of its residency's R2 bucket with their catalogues in the cell's database, and `bucketProvider.r2` provides them.

```ts
import { R2Bucket, R2BucketHost } from "@destack/bucket/r2";

const buckets = new R2BucketHost({
    files: new R2Bucket(environment.FILES),
    name: "destack-production-files-eu",
    catalogues,
    endpoint,
    region: "auto",
    credentials,
});
const provider = bucketProvider.r2(buckets);
```

## Sweeps

A provider's `controllers` hold the `SweepController`, which sweeps the host's open buckets daily and deletes what interrupted writes left.

```ts
provider.controllers; // [SweepController]
```

## Fences

A provider's `fence` refuses every write into a bucket while a transfer copies it, across restarts until `lift`, answering `UNAVAILABLE` and S3 `503 ServiceUnavailable`.

```ts
await provider.fence.fence(record);
await provider.fence.lift(record);
```

## Access

Each method of the provisioned `bucket` object requires `read` or `write`, and a bucket the system alone writes refuses other writers with `FORBIDDEN`.

```ts
await client.open({ mode: "write", ...file }); // ServiceError FORBIDDEN: the system alone writes bucket-…
```

## S3

`S3Server` serves the path-style S3 API over any `S3Bucket`, such as a `LocalBucket`.

```ts
import { S3Server } from "@destack/bucket/s3";

const server = new S3Server({
    region: "auto",
    credentials: async (accessKeyId) => keys.get(accessKeyId),
    open: async (bucketName, accessKeyId) => host.open(bucketName, accessKeyId),
    cors: [{ allowed: { origins: ["https://app.example"], methods: ["GET", "PUT"] } }],
});
const response = await server.fetch(request);
```

## Presigned URLs

`SignatureV4.presign` signs a URL for one request on one key until it expires, and any S3 client or plain `fetch` can use it.

```ts
import { S3Location, SignatureV4 } from "@destack/bucket/s3";

const location = { endpoint: new URL("https://files.example"), bucket: "files", region: "auto" };
const request = new Request(S3Location.url(location, "documents/readme.txt"));
const presigned = await new SignatureV4({ region: location.region }).presign(
    request,
    credentials,
    900,
    Date.now(),
);
```

## S3 coverage

`S3Server` implements the S3 subset that R2 implements.

```text
objects         GetObject, HeadObject, PutObject, DeleteObject, DeleteObjects, CopyObject, ListObjectsV2
multipart       CreateMultipartUpload, UploadPart, UploadPartCopy, CompleteMultipartUpload,
                AbortMultipartUpload, ListParts, ListMultipartUploads
authentication  SigV4 headers, presigned queries, aws-chunked bodies with signed chunks,
                or unsigned chunks and a trailing checksum
requests        conditional and copy-source conditional headers, ranges, CORS,
                STANDARD and STANDARD_IA storage classes
integrity       x-amz-content-sha256, Content-MD5, and CRC32, CRC32C and SHA-256 checksums
encryption      customer keys (SSE-C) on reads, writes and multipart uploads
```
