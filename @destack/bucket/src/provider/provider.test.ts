import { createHash } from "node:crypto";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { ResourceId } from "@destack/resource";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { aligned, schema, present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBucketHost } from "../local/index.ts";
import { segment } from "../catalogue/stack/index.ts";
import { StorageError } from "../error/index.ts";
import { bucketProvider } from "./provider.ts";
import { SweepController } from "./sweep.ts";

/** The bucket resource both hosts keep. */
const record = {
    id: ResourceId.parse("bucket-01996ab0-0000-7000-8000-000000000002"),
    scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
    spec: {},
    reference: null,
};

/** The credentials both hosts presign with. */
const CREDENTIALS = { accessKeyId: "TRANSFER", secretAccessKey: "transfer-secret" };

/** Open a temporary directory removed after the test. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-provider-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

test("provision a bucket once and destroy it with its files", async () => {
    const directory = await temporary();
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const provider = bucketProvider.local(buckets);

    // provision twice under one reference, keeping a file
    const provision = await provider.provision.provision(record);
    expect(await provider.provision.provision(record)).toEqual(provision);
    const bucket = await buckets.open({ bucketId: schema.identifier("bucket").parse(record.id) });
    await bucket.put("notes/a.txt", "first");
    expect(await readdir(directory)).toEqual([record.id]);

    // destroy the bucket's directory with its files
    await provider.provision.destroy({ ...record, ...provision });
    expect(await readdir(directory)).toEqual([]);
});

test("fence a bucket through its provider, and declare the sweep its host runs", async () => {
    const directory = await temporary();
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const provider = bucketProvider.local(buckets);
    const provisioned = { ...record, ...(await provider.provision.provision(record)) };
    const bucket = await buckets.open({ bucketId: schema.identifier("bucket").parse(record.id) });

    // refuse a write while fenced, and accept it once lifted
    await provider.fence.fence(provisioned);
    const refused = await bucket.put("notes/a.txt", "first").catch((error: unknown) => error);
    await provider.fence.lift(provisioned);
    await bucket.put("notes/a.txt", "first");
    expect({
        refused,
        written: await (await bucket.get("notes/a.txt"))?.text(),
        controllers: provider.controllers,
    }).toEqual({
        refused: new StorageError("FENCED", "bucket is fenced while a transfer copies it"),
        written: "first",
        controllers: [new SweepController(buckets)],
    });
});

test("open a bucket's catalogue under its space's scope, with the blobs its rows reference", async () => {
    const directory = await temporary();
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const provider = bucketProvider.local(buckets);
    const provision = await provider.provision.provision(record);
    const bucket = await buckets.open({ bucketId: schema.identifier("bucket").parse(record.id) });
    await bucket.put("notes/a.txt", "first");

    // read the file's segment, logged under the space, and its blob's bytes
    const handle = await provider.open.open({ ...record, ...provision }, []);
    onTestFinished(() => handle.close());
    const stored = aligned(await handle.database.select().from(segment), 0);
    const changes = await handle.database.log.read({ tables: [segment], after: 0 });
    const read: Uint8Array[] = [];
    for await (const chunk of present(handle.blobs, "blobs").read(stored.blob)) {
        read.push(chunk);
    }
    expect([
        stored.blob,
        changes.changes.map((change) => change.scope),
        new TextDecoder().decode(Buffer.concat(read)),
    ]).toEqual([createHash("sha256").update("first").digest("hex"), [record.scope], "first"]);
});
