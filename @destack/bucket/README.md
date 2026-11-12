# @destack/bucket

Store files in buckets on local disk, Cloudflare R2 or any S3 endpoint.

## Overview

A package declares the buckets it uses, and the host binds each to a directory, an R2 bucket or an S3 endpoint.

- **Bucket**: files by key; `put`, `get`, `head`, `list` and `delete`, with R2's `onlyIf` preconditions and S3 Object Lock's `retainUntil`.
- **File**: one stored body with its `etag`, HTTP metadata and custom metadata; a `put` replaces it whole.
- **Multipart upload**: a large file sent in numbered parts; `uploadPart`, `complete` and `abort`.

```ts
import type { Bucket } from "@destack/bucket";
import { defineBucket } from "@destack/bucket/declare";

export const files = defineBucket({ name: "files" });

async (bucket: Bucket) => {
    const httpMetadata = { contentType: "text/plain" };
    const stored = await bucket.put("notes/first.txt", "Hello", { httpMetadata });
    const file = await bucket.get("notes/first.txt");
    const text = await file?.text();
    const listing = await bucket.list({ prefix: "notes/" });
    await bucket.delete("notes/first.txt");
};
```

## Examples

### Replace a file only if unchanged

`onlyIf` rejects a write with `PRECONDITION_FAILED` when the stored file changed since it was read.

```ts
import type { Bucket } from "@destack/bucket";

async (bucket: Bucket, body: string) => {
    const current = await bucket.head("index.json");
    const onlyIf = current ? { etagMatches: current.etag } : { etagDoesNotMatch: "*" };
    await bucket.put("index.json", body, { onlyIf });
};
```

### Upload a large file in parts

A multipart upload sends numbered parts, each but the last at least 5 MiB, then assembles them into one file.

```ts
import type { Bucket } from "@destack/bucket";

async (bucket: Bucket, parts: Uint8Array<ArrayBuffer>[]) => {
    const upload = await bucket.createMultipartUpload("media/video.mp4");
    const uploaded = [];
    for (const [index, part] of parts.entries()) {
        uploaded.push(await upload.uploadPart(index + 1, part));
    }
    await upload.complete(uploaded);
};
```

### Keep a bucket in a directory

`LocalBucket` keeps a bucket in a directory of an environment's files.

```ts
import { LocalBucket } from "@destack/bucket/bun";
import { BunEnvironment } from "@destack/environment/bun";

await using bucket = await LocalBucket.open("/var/files", "space-1", BunEnvironment.system);
await bucket.put("hello.txt", "Hello");
```

### Call a bucket over S3

`S3Client` is a bucket over the path-style S3 API, signing each request with Signature Version 4.

```ts
import { S3Client } from "@destack/bucket/s3";
import { BunEnvironment } from "@destack/environment/bun";

const send = (request: Request) => fetch(request);
const bucket = new S3Client("https://s3.example.com/files", send, BunEnvironment.system.crypto);
await bucket.put("notes/first.txt", "Hello");
```

### Test code that stores files

`MemoryBucket` keeps files in memory on a clock the test sets.

```ts
import { MemoryBucket } from "@destack/bucket/test";
import { testSimulation } from "@destack/test/world";

await using simulation = testSimulation();
const bucket = new MemoryBucket(simulation.clock);
await bucket.put("a.txt", "a");
```

## Comparison

### Cloudflare R2

The `Bucket` interface follows R2's Workers binding; Destack's differs in three ways.

- `retainUntil` locks a file against replacement and deletion, as S3 Object Lock does in compliance mode.
- The same interface runs over a local directory, R2, any S3 endpoint and memory, so tests and machines use the code that production does.
- `S3Server` serves any bucket over the path-style S3 API, so S3 tools reach a local bucket too.
