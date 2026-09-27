Declare and access file storage in Destack.

## Usage

```ts
import { defineBucket } from "@destack/bucket";

export const files = defineBucket({
    name: "files",
    spec: {},
});
```

```ts
const bucket = files.get(context);
await bucket.put("documents/readme.txt", "Hello", { httpMetadata: { contentType: "text/plain" } });
const document = await bucket.get("documents/readme.txt");
const text = document ? await document.text() : undefined;
```

```ts
const upload = await bucket.createMultipartUpload("media/video.mp4", {
    httpMetadata: { contentType: "video/mp4" },
});
const first = await upload.uploadPart(1, firstChunk);
const last = await upload.uploadPart(2, lastChunk);
await upload.complete([first, last]);
```

## Hosts

```ts
import { LocalBucket } from "@destack/bucket/local";
import { ResourceContext } from "@destack/resource";

await using bucket = await LocalBucket.open("./resources/files");
const context = new ResourceContext();
context.bind(files, bucket);
```

```ts
import { R2Bucket } from "@destack/bucket/r2";

context.bind(files, new R2Bucket(environment.FILES));
```

## Access

Roles in a space grant the `bucket` policy's permissions on each bucket, which `bucketDatabase` records by resource.

| Permission | Grants |
|---|---|
| `read`, `update`, `delete` | Administration of the bucket resource |
| `list` | File keys and metadata |
| `download`, `upload`, `remove` | File bodies and their removal |

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
const url = await new SignatureV4({ region: location.region }).presign(request, credentials, 900, Date.now());
```

The server covers R2's S3-compatible subset.

| Area | Supported |
|---|---|
| Objects | GetObject, HeadObject, PutObject, DeleteObject, DeleteObjects, CopyObject, ListObjectsV2 |
| Multipart | CreateMultipartUpload, UploadPart, UploadPartCopy, CompleteMultipartUpload, AbortMultipartUpload, ListParts, ListMultipartUploads |
| Authentication | SigV4 headers, presigned queries, `aws-chunked` bodies with signed chunks, or unsigned chunks and a trailing checksum |
| Requests | conditional and copy-source conditional headers, ranges, CORS, `STANDARD` and `STANDARD_IA` storage classes |
| Integrity | `x-amz-content-sha256`, `Content-MD5`, and CRC32, CRC32C and SHA-256 checksums |
