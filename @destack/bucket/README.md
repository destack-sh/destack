# @destack/bucket

Store files in buckets on local disk, Cloudflare R2 or any S3 endpoint.

## Overview

`@destack/bucket` is R2's bucket API on every host: local directories, R2, S3 endpoints and memory, plus an S3 server.

Your package declares a bucket and stores files by key:

- `defineBucket`: declares a bucket resource.
- `Bucket`: files by key, with `put`, `get`, `head`, `list` and `delete`, as in R2.
- `BucketFile` and `BucketFileBody`: one stored file's metadata, and the file with its body.
- `BucketCondition`: a read's or write's precondition, as R2's `onlyIf`.
- `MultipartUpload`: a large file sent in parts.

Backends:

- `LocalBucket`: a bucket in a directory.
- `R2Bucket`: a bucket over a Cloudflare R2 binding.
- `S3Client`: a bucket over the path-style S3 API, signed with Signature Version 4.
- `MemoryBucket`: a bucket in memory, for tests.
- `S3Server`: serves buckets over the path-style S3 API.

Declare a bucket, then store, read, list and delete files:

```ts
import type { Bucket } from "@destack/bucket";
import { defineBucket } from "@destack/bucket/declare";

// declare the package's bucket
export const files = defineBucket({ name: "files" });

/** Store a note, read it back, list the notes, and delete it. */
export async function storeNote(bucket: Bucket) {
    // store the note with its content type
    const httpMetadata = { contentType: "text/plain" };
    await bucket.put("notes/first.txt", "Hello", { httpMetadata });

    // read it, and list the notes
    const file = await bucket.get("notes/first.txt");
    const text = await file?.text();
    const listing = await bucket.list({ prefix: "notes/" });

    // delete it
    await bucket.delete("notes/first.txt");

    return { text, keys: listing.files.map((entry) => entry.key) };
}
```

## Reference

### Bucket

A `Bucket` keeps files by key, each key 1 to 1024 UTF-8 bytes without NUL.
A `put` replaces a file whole and at once, and a body is a string, bytes, a `Blob` or a stream.

| Verb                                   | What it does                                                                                                  |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `head(key)`                            | reads a file's metadata, or `null` for a missing key                                                          |
| `get(key, { onlyIf, range, ssecKey })` | reads a file with its body, or `null` for a missing key, a `range` being `{ offset, length }` or `{ suffix }` |
| `put(key, body, options)`              | stores a file, replacing any earlier one                                                                      |
| `delete(key)`, `delete(keys)`          | deletes one file or up to 1000, passing missing keys                                                          |
| `list(options)`                        | lists a page of files in key order                                                                            |
| `createMultipartUpload(key, options)`  | starts a multipart upload                                                                                     |
| `resumeMultipartUpload(key, uploadId)` | continues a multipart upload, checking it exists on its next call                                             |

`put` takes these options, and `createMultipartUpload` takes the same without `onlyIf` and the digests:

| Option                  | What it does                                                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `onlyIf`                | refuses the write unless the stored file, or its absence, meets a condition                                        |
| `httpMetadata`          | stores `contentType`, `contentLanguage`, `contentDisposition`, `contentEncoding`, `cacheControl` and `cacheExpiry` |
| `customMetadata`        | stores names and values of your own, at most 2 KiB together                                                        |
| `md5`, `sha1`, `sha256` | refuses a body unlike the digest, given as hexadecimal text or bytes                                               |
| `storageClass`          | `"Standard"`, the default, or `"InfrequentAccess"`                                                                 |
| `ssecKey`               | encrypts the file with your key, 32 bytes or 64 hexadecimal digits                                                 |
| `retainUntil`           | locks the file against replacement and deletion until a time                                                       |

`list` takes these options, and answers `{ files, delimitedPrefixes, truncated, cursor }`:

| Option       | What it does                                            |
| ------------ | ------------------------------------------------------- |
| `prefix`     | lists only keys with the prefix                         |
| `delimiter`  | groups the keys after the prefix up to the delimiter    |
| `cursor`     | continues from the page before, with the same prefix    |
| `startAfter` | lists only keys after a key, unless a `cursor` is given |
| `limit`      | lists at most 1 to 1000 files                           |

### BucketCondition

Conditions work as HTTP's conditional requests: an entity tag condition overrides a date condition.
A failed condition throws `PRECONDITION_BLOCKED`, with the stored file as `current`.

| Condition            | Holds when                                                  |
| -------------------- | ----------------------------------------------------------- |
| `etagMatches`        | the file's entity tag is the given one, or any file for `*` |
| `etagDoesNotMatch`   | the file's entity tag differs, or no file exists for `*`    |
| `uploadedBefore`     | the file was uploaded before a time                         |
| `uploadedAfter`      | the file was uploaded after a time                          |
| `secondsGranularity` | compares upload times in whole seconds, as HTTP dates do    |

### BucketFile

A `BucketFile` describes one upload of a file, and a `BucketFileBody` adds its body.

| Field or verb                                            | What it is                                                  |
| -------------------------------------------------------- | ----------------------------------------------------------- |
| `key`, `size`, `uploaded`                                | the file's key, its length in bytes and its upload time     |
| `version`                                                | the upload's identifier                                     |
| `etag`, `httpEtag`                                       | the entity tag, unquoted and quoted for HTTP                |
| `httpMetadata`, `customMetadata`                         | the stored HTTP headers and your own metadata               |
| `digest`, `storageClass`, `retainUntil`                  | the stored digests, the storage class and the lock's end    |
| `writeHttpMetadata(headers)`                             | writes the stored HTTP headers to a response's headers      |
| `body`, `range`                                          | the body's stream and the range read, on a `BucketFileBody` |
| `text()`, `json()`, `bytes()`, `arrayBuffer()`, `blob()` | read the body, on a `BucketFileBody`                        |

### MultipartUpload

A multipart upload has up to 10000 parts, each but the last at least 5 MiB.

| Verb                           | What it does                                      |
| ------------------------------ | ------------------------------------------------- |
| `uploadPart(partNumber, body)` | stores one part, answering `{ partNumber, etag }` |
| `complete(parts)`              | assembles the given parts in order into one file  |
| `abort()`                      | discards the upload and its parts                 |

### Backends

Every backend is a `Bucket`.
`LocalBucket` also has S3's `copy`, `listUploads`, `uploadPartCopy` and `listParts`, which `S3Server` serves.
`LocalBucket.open` creates the bucket in a scope when the directory has none, and an `undefined` scope requires an existing bucket.

| Backend                                               | Entry                        | What it keeps                                     |
| ----------------------------------------------------- | ---------------------------- | ------------------------------------------------- |
| `LocalBucket.open(directory, scope, environment)`     | `@destack/bucket/bun`        | a SQLite catalogue and the files in a directory   |
| `new R2Bucket(binding)`                               | `@destack/bucket/cloudflare` | the files of an R2 binding                        |
| `S3Client.of({ location, credentials }, environment)` | `@destack/bucket/s3`         | the files of a bucket at an S3 endpoint, signed   |
| `new S3Client(url, fetch, environment)`               | `@destack/bucket/s3`         | the files at a bucket's URL, sent through a fetch |
| `new MemoryBucket(clock)`                             | `@destack/bucket/test`       | the files in memory, without multipart uploads    |

### S3Server

`new S3Server(options)` serves buckets over the path-style S3 API.
`fetch(request)` checks Signature Version 4, and `serve(request, accessKeyId)` answers an already authorized request.

| Option        | What it is                                                            |
| ------------- | --------------------------------------------------------------------- |
| `region`      | the region clients sign requests for                                  |
| `credentials` | a function answering an access key's credentials, or `undefined`      |
| `open`        | a function opening a bucket by name for an access key, or `undefined` |
| `cors`        | the CORS rules of every bucket                                        |
| `environment` | the clock, WebCrypto and randomness it checks requests with           |

## Examples

### Replace a file only if unchanged

`onlyIf` refuses a write when the file changed since it was read.

```ts
import { type Bucket, BucketError } from "@destack/bucket";

/** Write the index, unless another writer changed it since it was read. */
export async function writeIndex(bucket: Bucket, body: string): Promise<boolean> {
    // require the file as read, or no file at all
    const current = await bucket.head("index.json");
    const onlyIf = current ? { etagMatches: current.etag } : { etagDoesNotMatch: "*" };

    try {
        await bucket.put("index.json", body, { onlyIf });

        return true;
    } catch (error) {
        // report a lost race, and throw any other failure
        if (BucketError.is(error, "PRECONDITION_BLOCKED")) {
            return false;
        }
        throw error;
    }
}
```

### Upload a large file in parts

`abort` discards the parts of a failed upload.

```ts
import type { Bucket, UploadedPart } from "@destack/bucket";

/** Upload a video in parts of at least 5 MiB each but the last. */
export async function uploadVideo(bucket: Bucket, parts: Uint8Array<ArrayBuffer>[]) {
    const upload = await bucket.createMultipartUpload("media/video.mp4", {
        httpMetadata: { contentType: "video/mp4" },
    });

    try {
        // upload each numbered part
        const uploaded: UploadedPart[] = [];
        for (const [index, part] of parts.entries()) {
            uploaded.push(await upload.uploadPart(index + 1, part));
        }

        return await upload.complete(uploaded);
    } catch (error) {
        // discard the parts sent so far
        await upload.abort();
        throw error;
    }
}
```

### Keep a bucket in a directory

`LocalBucket.open` creates a bucket in a directory, or opens the one already there.

```ts
import { LocalBucket } from "@destack/bucket/bun";
import { BunEnvironment } from "@destack/environment/bun";

// open the space's bucket, and store a file
await using bucket = await LocalBucket.open("/var/files", "space-1", BunEnvironment.system);
await bucket.put("hello.txt", "Hello");
```

### Call a bucket over S3

`S3Client.of` signs each request for the bucket's region with its credentials.

```ts
import { S3Client, S3Location } from "@destack/bucket/s3";
import { BunEnvironment } from "@destack/environment/bun";

// address the bucket and its credentials
const location = S3Location.parse({
    endpoint: "https://s3.example.com",
    bucket: "files",
    region: "eu-central-1",
});
const credentials = { accessKeyId: "AKIDEXAMPLE", secretAccessKey: "secret" };

// store a file through the S3 API
const bucket = S3Client.of({ location, credentials }, BunEnvironment.system);
await bucket.put("notes/first.txt", "Hello");
```

### Test code that stores files

`MemoryBucket` keeps files in memory, on a simulation's clock.

```ts
import { MemoryBucket } from "@destack/bucket/test";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("store a note in memory", async () => {
    // store a note in a bucket on the simulation's clock
    await using simulation = testSimulation();
    const bucket = new MemoryBucket(simulation.environment("host").clock);
    await bucket.put("a.txt", "a");

    expect(await (await bucket.get("a.txt"))?.text()).toBe("a");
});
```

## Comparison

### Cloudflare R2

The `Bucket` interface basically follows R2's Workers binding, with the same verbs, options and conditions.
It adds S3 Object Lock's `retainUntil`.

| R2                                               | `@destack/bucket`              | Difference                                                                          |
| ------------------------------------------------ | ------------------------------ | ----------------------------------------------------------------------------------- |
| `R2Bucket`                                       | `Bucket`                       | The same, on every host.                                                            |
| `head`, `get`, `put`, `delete`, `list`           | the same                       | The same.                                                                           |
| `R2Object`, `R2ObjectBody`                       | `BucketFile`, `BucketFileBody` | The same, plus `digest` and `retainUntil`.                                          |
| `onlyIf` on `get`                                | `onlyIf` on `get`              | Throws `PRECONDITION_BLOCKED` instead of answering a file without a body.           |
| `R2Conditional`                                  | `BucketCondition`              | The same, with entity tags unquoted.                                                |
| `createMultipartUpload`, `resumeMultipartUpload` | the same                       | The same.                                                                           |
| `ssecKey`, `storageClass`                        | the same                       | The same.                                                                           |
| none                                             | `retainUntil`                  | Locks a file against replacement and deletion, as S3 Object Lock's compliance mode. |
| R2's S3 API                                      | `S3Server`                     | Serves any bucket, so S3 tools reach a local one too.                               |
