import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { MemoryKeychain } from "@destack/host/keychain";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { S3Location, S3Server, SignatureV4 } from "../s3/index.ts";
import { LocalBucketHost } from "./host.ts";

test("serve a host's buckets over S3 by name, presigned with credentials its keychain keeps", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-buckets-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    // generate the host's credentials once
    const keychain = new MemoryKeychain();
    const credentials = await LocalBucketHost.credentials(keychain, "s3/host-1");
    expect([
        await LocalBucketHost.credentials(keychain, "s3/host-1"),
        credentials.accessKeyId.length,
        credentials.secretAccessKey.length,
    ]).toEqual([credentials, 20, 40]);

    // serve the buckets the host keeps, and no other name
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        credentials,
    });
    const server = new S3Server({
        region: "local",
        credentials: async (id) => (id === credentials.accessKeyId ? credentials : undefined),
        open: (name) => buckets.named(name),
    });
    const reference = {
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
        bucketId: schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-000000000002"),
    };
    const other = schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-000000000003");
    expect([await buckets.named(other), await buckets.named("files")]).toEqual([
        undefined,
        undefined,
    ]);
    await (await buckets.open(reference, "space-test")).put("notes/a.txt", "first");

    // read the file through a URL presigned at the located endpoint
    const { location } = await buckets.locate(reference);
    const presigned = await new SignatureV4({ region: location.region }).presign(
        new Request(S3Location.url(location, "notes/a.txt").href),
        credentials,
        60,
        Date.now(),
    );
    const response = await server.fetch(presigned);
    expect([new URL(presigned.url).origin, response.status, await response.text()]).toEqual([
        "http://s3.localhost:4200",
        200,
        "first",
    ]);
});
