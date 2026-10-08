# @destack/bucket

Store files in buckets: the `Bucket` contract, its Bun, Cloudflare and memory hosts, their provider, and the S3 API over them.

## Declarations

`defineBucket` declares a bucket by name, and `Bucket` is the file storage a host binds it to.

```ts
import { defineBucket } from "@destack/bucket/declare";

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

### Preconditions

`onlyIf` makes a read or write conditional, and a failed condition throws `BucketError` `PRECONDITION_FAILED` with the file it met.

```ts
await bucket.put("index.json", body, { onlyIf: { etagMatches: previous.etag } }); // PRECONDITION_FAILED once another write won
error.current; // the file the condition met, null where none exists
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

## Object lock

`retainUntil` locks a file in compliance mode until a date, as S3 Object Lock does.

```ts
await bucket.put("audit/2026-10/segment.parquet", body, {
    retainUntil: new Date(sealedAt + SEVEN_YEARS),
});
await bucket.delete("audit/2026-10/segment.parquet"); // LOCKED
(await bucket.head("audit/2026-10/segment.parquet"))?.retainUntil; // the date
```

## Errors

A storage failure throws a `BucketError` with a stable code.

```ts
import { BucketError } from "@destack/bucket/error";

new BucketError("FENCED", "bucket is fenced while a transfer copies it").toServiceError(); // { code: "SERVICE_UNAVAILABLE", message: "bucket is fenced while a transfer copies it" }
```

## Memory buckets

`MemoryBucket` keeps files in memory with a host's conditions, ranges, object locks and listings, on a clock the test sets.

```ts
import { MemoryBucket } from "@destack/bucket/test";

const bucket = new MemoryBucket(() => now);
```

## Hosts

A bucket host stores buckets and serves them over S3.

### Local buckets

`LocalBucket.open` opens a bucket in a local directory.

```ts
import { LocalBucket } from "@destack/bucket/bun";
import { ResourceContext } from "@destack/resource/context";

await using bucket = await LocalBucket.open("./resources/files");
const context = new ResourceContext();
context.bind(files, bucket);
```

### Machine hosts

`LocalBucketHost` stores a machine's buckets in local directories.

```ts
import { LocalBucketHost } from "@destack/bucket/bun";
import { bucketProvider } from "@destack/bucket/provider";
import { S3Credentials } from "@destack/bucket/s3";

const buckets = new LocalBucketHost({
    directory,
    endpoint,
    region: "local",
    credentials: async (space) => S3Credentials.derive(await root(space), space),
});
const provider = bucketProvider(buckets, serveBuckets(buckets)); // serveBuckets from @destack/space/server
```

### Operations

`R2Bucket.takeOperations` takes the operations a binding sent since the last take, by R2's billing class.

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

### Presigned URLs

`S3Signature.presign` signs a URL for one request on one key until it expires (Signature Version 4).

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
```

### Customer keys

`customerKeyHeaders` writes the SSE-C headers of a customer key, which `S3Server` encrypts and decrypts with.

```ts
import { customerKeyHeaders } from "@destack/bucket/s3";

await fetch(url, { method: "PUT", body, headers: customerKeyHeaders(key) });
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
const provider = bucketProvider(buckets, serveBuckets(buckets)); // serveBuckets from @destack/space/server
```
