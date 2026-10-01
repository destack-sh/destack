Declare and access file storage in Destack.

## Usage

A package declares a bucket by name, and the host provides it.

```ts
import { defineBucket } from "@destack/bucket";

export const files = defineBucket({
    name: "files",
    spec: {},
});
```

A handler reads and writes files through its bucket.

```ts
const bucket = files.get(context);
await bucket.put("documents/readme.txt", "Hello", { httpMetadata: { contentType: "text/plain" } });
const document = await bucket.get("documents/readme.txt");
const text = document ? await document.text() : undefined;
```

Large files upload in parts.

```ts
const upload = await bucket.createMultipartUpload("media/video.mp4", {
    httpMetadata: { contentType: "video/mp4" },
});
const first = await upload.uploadPart(1, firstChunk);
const last = await upload.uploadPart(2, lastChunk);
await upload.complete([first, last]);
```

A customer key encrypts a file's content, and every read of it sends the same key.

```ts
await bucket.put("private/report.pdf", report, { ssecKey });
const sealed = await bucket.get("private/report.pdf", { ssecKey });
```

## Hosts

A host binds each declared bucket to a local directory or an R2 bucket.

```ts
import { LocalBucket } from "@destack/bucket/local";
import { ResourceContext } from "@destack/resource/context";

await using bucket = await LocalBucket.open("./resources/files");
const context = new ResourceContext();
context.bind(files, bucket);
```

On Workers, the host binds the R2 bucket of its environment.

```ts
import { R2Bucket } from "@destack/bucket/r2";

context.bind(files, new R2Bucket(environment.FILES));
```

A device host keeps its buckets in `LocalBucketHost`, serves them over S3, and provides them to its spaces.

```ts
import { LocalBucketHost } from "@destack/bucket/local";
import { localBucketProvider } from "@destack/bucket/provider";
import { S3Server } from "@destack/bucket/s3";

const credentials = await LocalBucketHost.credentials(keychain, `s3/${hostId}`);
const buckets = new LocalBucketHost({
    directory,
    endpoint: new URL(`http://s3.localhost:${port}`),
    region: "local",
    credentials,
});
const s3 = new S3Server({
    region: "local",
    credentials: async (id) => (id === credentials.accessKeyId ? credentials : undefined),
    open: (name) => buckets.named(name),
});
const provider = localBucketProvider(buckets);
```

## Access

Roles in a space grant the `bucket` object's permissions on each bucket.

| Permission | Grants |
|---|---|
| `get`, `list` | The bucket records |
| `files` | File keys and metadata |
| `download` | `open` in `read` mode: a presigned lease on a file's body |
| `upload` | `open` in `write` mode, and multipart uploads |
| `remove` | Removing files |

## S3

`S3Server` serves the S3 API path-style over any `S3Bucket`, such as a `LocalBucket`, while R2 serves it natively.

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

A presigned URL grants one request on one key until it expires, and any S3 client or plain `fetch` can follow it.

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

The server covers R2's S3-compatible subset.

| Area | Supported |
|---|---|
| Objects | GetObject, HeadObject, PutObject, DeleteObject, DeleteObjects, CopyObject, ListObjectsV2 |
| Multipart | CreateMultipartUpload, UploadPart, UploadPartCopy, CompleteMultipartUpload, AbortMultipartUpload, ListParts, ListMultipartUploads |
| Authentication | SigV4 headers, presigned queries, `aws-chunked` bodies with signed chunks, or unsigned chunks and a trailing checksum |
| Requests | conditional and copy-source conditional headers, ranges, CORS, `STANDARD` and `STANDARD_IA` storage classes |
| Integrity | `x-amz-content-sha256`, `Content-MD5`, and CRC32, CRC32C and SHA-256 checksums |
| Encryption | customer keys (SSE-C) on reads, writes and multipart uploads |
