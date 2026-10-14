# @destack/bucket

`Bucket` is Cloudflare R2's Workers binding API (`put`, `get`, `head`, `onlyIf`, `httpMetadata`, `createMultipartUpload`, `ssecKey`) with S3 Object Lock's `retainUntil`, hosted in local directories by `/bun`, in R2 by `/cloudflare` and in memory by `/test`, and `/s3` serves any bucket over the path-style S3 API with Signature Version 4 and SSE-C.

```ts
// R2Bucket.put
await bucket.put("documents/readme.txt", "Hello", { httpMetadata: { contentType: "text/plain" } });
// R2's onlyIf, PRECONDITION_FAILED
await bucket.put("index.json", body, { onlyIf: { etagMatches: previous.etag } });
// S3 Object Lock in compliance mode
await bucket.put("audit/2026-10/segment.parquet", body, { retainUntil });
const upload = await bucket.createMultipartUpload("media/video.mp4"); // R2's multipart upload
// an S3 presigned URL
await new S3Signature({ region: "auto" }).presign(request, credentials, 900, Date.now());
```

## Declarations

`defineBucket` declares a bucket by name, and `files.get(context)` returns the bucket a host binds it to.

```ts
import { defineBucket } from "@destack/bucket/declare";

export const files = defineBucket({ name: "files", spec: {} });
const document = await files.get(context).get("documents/readme.txt");
const text = document ? await document.text() : undefined;
```

## Hosts

`LocalBucket` keeps a bucket in a local directory, `R2Bucket` wraps a Workers R2 binding, and `MemoryBucket` keeps files in memory on a clock a test sets.

```ts
import { LocalBucket } from "@destack/bucket/bun";
import { R2Bucket } from "@destack/bucket/cloudflare";
import { MemoryBucket } from "@destack/bucket/test";

await using local = await LocalBucket.open("./resources/files");
const remote = new R2Bucket(environment.FILES);
remote.takeOperations(); // { classA: 1, classB: 1 }, R2's billing classes
const memory = new MemoryBucket(() => now);
```

## Providers

`bucketProvider` manages a machine's buckets as resources: it snapshots, fences and sweeps them.

```ts
import { LocalBucketHost } from "@destack/bucket/bun";
import { bucketProvider } from "@destack/bucket/provider";

const buckets = new LocalBucketHost({ directory, endpoint, region: "local", credentials });
const provider = bucketProvider(buckets, serveBuckets(buckets));
```

## S3

`S3Server` serves the path-style S3 API over any `S3Bucket`, and `S3Client` is a `Bucket` over it, as a workload calls its bound buckets at its machine's egress.

```ts
import { S3Client, S3Server } from "@destack/bucket/s3";

const server = new S3Server({
    region: "auto",
    credentials: async (accessKeyId) => keys.get(accessKeyId),
    open: async (bucketName, accessKeyId) => host.open(bucketName, accessKeyId),
    cors: [{ allowed: { origins: ["https://app.example"], methods: ["GET", "PUT"] } }],
});
const response = await server.fetch(request); // SigV4, or server.serve(request, accessKeyId) once authorized
const files = new S3Client(`${egress}/${bucketId}`, fetch);
await files.put("notes/first.txt", "Hello", { sha256 }); // PUT with x-amz-checksum-sha256
```
