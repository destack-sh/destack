# @destack/bucket

Declare file storage, read and write its files, and serve it over S3.

## Declarations

`defineBucket` declares a bucket by name.

```ts
import { defineBucket } from "@destack/bucket";

export const files = defineBucket({ name: "files", spec: {} });
export const build = defineBucket({ name: "build", spec: { write: "system" } });
```

### Files

`files.get(context)` returns the bound bucket for reading and writing files.

```ts
const bucket = files.get(context);
await bucket.put("documents/readme.txt", "Hello", { httpMetadata: { contentType: "text/plain" } });
const document = await bucket.get("documents/readme.txt");
const text = document ? await document.text() : undefined;
```

### Multipart uploads

`createMultipartUpload` starts an upload in parts.

```ts
const upload = await bucket.createMultipartUpload("media/video.mp4", {
    httpMetadata: { contentType: "video/mp4" },
});
const first = await upload.uploadPart(1, firstChunk);
const last = await upload.uploadPart(2, lastChunk);
await upload.complete([first, last]);
```

### Encryption

`ssecKey` encrypts a file with a customer key that every read must pass.

```ts
await bucket.put("private/report.pdf", report, { ssecKey });
const sealed = await bucket.get("private/report.pdf", { ssecKey });
```

## Objects

`serveBuckets` serves the `bucket` objects of a host's buckets.

```ts
import { serveBuckets } from "@destack/bucket/server";

const handled = serveBuckets(host);
await client.open({ mode: "write", ...file }); // ServiceError FORBIDDEN: the system alone writes bucket-…
```

## Service

`implementBucket` serves the bucket objects and runs the providers' controllers.

```ts
import { implementBucket } from "@destack/bucket/server";
import { bucketDatabase } from "@destack/bucket/stack";

const buckets = implementBucket({ database, callKey, directory, providers, machine, cell, spaces });
const source = buckets.objects; // a space creates a bucket by a call this service runs
```

## Hosts

A bucket host stores buckets and serves them over S3.

### Local buckets

`LocalBucket.open` opens a bucket in a local directory.

```ts
import { LocalBucket } from "@destack/bucket/local";
import { ResourceContext } from "@destack/resource/context";

await using bucket = await LocalBucket.open("./resources/files");
const context = new ResourceContext();
context.bind(files, bucket);
```

### Device hosts

`LocalBucketHost` stores a device host's buckets in local directories.

```ts
import { LocalBucketHost } from "@destack/bucket/local";
import { bucketProvider } from "@destack/bucket/provider";
import { S3Credentials } from "@destack/bucket/s3";

const buckets = new LocalBucketHost({
    directory,
    endpoint,
    region: "local",
    credentials: async (space) => S3Credentials.derive(await root(space), space),
});
const provider = bucketProvider(buckets);
```

### Operations

`R2Bucket.takeOperations` takes the operations a binding sent since the last take, in the classes R2 bills: Class A for writes, lists and multipart steps, Class B for reads, deletes free.

```ts
const files = new R2Bucket(environment.FILES);
await files.put("note", "first");
await files.get("note");
files.takeOperations(); // { classA: 1, classB: 1 }
```

### Snapshots

`snapshot` copies a bucket's catalogue and files into a content-addressed store.

```ts
const digest = await provider.snapshot.snapshot(record, [], store);
await target.snapshot.restore(provisioned, [], digest, store);
```

### Sweeps

`SweepController` deletes the files that interrupted writes left, once a day.

```ts
provider.controllers; // [SweepController]
```

### Fences

`fence` refuses every write to a bucket while a transfer copies it.

```ts
await provider.fence.fence(record);
await provider.fence.lift(record);
```

## Object lock

`retainUntil` locks a file in compliance mode, as S3 Object Lock does: nobody replaces or deletes it before the date, and R2's own buckets lock files by bucket lock rules instead.

```ts
await bucket.put("audit/2026-10/segment.parquet", body, {
    retainUntil: new Date(sealedAt + SEVEN_YEARS),
});
await bucket.delete("audit/2026-10/segment.parquet"); // BucketError LOCKED until the date, S3 AccessDenied
(await bucket.head("audit/2026-10/segment.parquet"))?.retainUntil; // the date
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

### Spaces

`serveS3` serves a cell's buckets: each space signs with credentials its root secret derives and reaches only its own buckets.

```ts
import { serveS3 } from "@destack/bucket/server";

const server = serveS3(buckets, database); // database: the bucket service's, keeping each bucket's space
await S3Credentials.derive(root, spaceId); // { accessKeyId: spaceId, secretAccessKey: HKDF-SHA256 output }
```

### Presigned URLs

`S3Signature.presign` signs a URL for one request on one key until it expires, with the upstream Signature Version 4 signer over WebCrypto on Bun, workerd and browsers.

```ts
import { S3Location, S3Signature } from "@destack/bucket/s3";

const location = { endpoint: new URL("https://files.example"), bucket: "files", region: "auto" };
const request = new Request(S3Location.url(location, "documents/readme.txt"));
const presigned = await new S3Signature({ region: location.region }).presign(
    request,
    credentials,
    900,
    Date.now(),
);
// signed by AwsSigner from @destack/identity/aws, keeping each access key's daily signing key
// verification signs the request's signed parts again and compares the signatures in constant time
```

### Coverage

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

## Cloudflare

`@destack/bucket/cloudflare` stores files in R2.

### Workers

`R2Bucket` wraps an R2 bucket binding of a Workers environment.

```ts
import { R2Bucket } from "@destack/bucket/cloudflare";

context.bind(files, new R2Bucket(environment.FILES));
```

### Cells

`R2BucketHost` stores a cell's buckets as prefixes of one R2 bucket.

```ts
import { R2Bucket, R2BucketHost } from "@destack/bucket/cloudflare";
import { bucketProvider } from "@destack/bucket/provider";

const buckets = new R2BucketHost({
    files: new R2Bucket(environment.FILES),
    name: "destack-production-files-eu",
    catalogues,
    endpoint,
    region: "auto",
    credentials,
});
const provider = bucketProvider(buckets);
```

## Errors

A storage failure throws a `BucketError` with a stable code.

```ts
import { BucketError } from "@destack/bucket/error";

new BucketError("FENCED", "bucket is fenced while a transfer copies it").toServiceError(); // { code: "SERVICE_UNAVAILABLE", message: "bucket is fenced while a transfer copies it" }
```

## Tests

`BucketFixture` provisions a space with a local bucket served over S3.

```ts
import { BucketFixture } from "@destack/bucket/test";

await using fixture = await BucketFixture.open(database);
const response = await fixture.transfer(upload, "Hello, bucket");
```
