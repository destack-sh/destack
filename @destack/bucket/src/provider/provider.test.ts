import { createHash } from "node:crypto";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { ResourceId } from "@destack/resource";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { identifier } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBucketHost } from "../local/index.ts";
import { segment } from "../local/stack/index.ts";
import { Feed, Replica, replicaTables } from "@destack/sync";
import { ZoneTransfer } from "@destack/space/server";
import { localBucketProvider } from "./provider.ts";

/** The bucket resource both hosts keep. */
const record = {
    id: ResourceId.parse("bucket-01996ab0-0000-7000-8000-000000000002"),
    scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
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
    const provider = localBucketProvider(buckets);

    // provision twice under one reference, keeping a file
    const provision = await provider.provision(record);
    expect(await provider.provision(record)).toEqual(provision);
    const bucket = await buckets.open({ bucketId: identifier("bucket").parse(record.id) });
    await bucket.put("notes/a.txt", "first");
    expect(await readdir(directory)).toEqual([record.id]);

    // destroy the bucket's directory with its files
    await provider.destroy({ ...record, ...provision });
    expect(await readdir(directory)).toEqual([]);
});

test("open a bucket's catalogue under its space's scope, with the blobs its rows reference", async () => {
    const directory = await temporary();
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const provider = localBucketProvider(buckets);
    const provision = await provider.provision(record);
    const bucket = await buckets.open({ bucketId: identifier("bucket").parse(record.id) });
    await bucket.put("notes/a.txt", "first");

    // read the file's segment, logged under the space, and its blob's bytes
    const handle = await provider.open({ ...record, ...provision }, []);
    onTestFinished(() => handle.close());
    const [stored] = await handle.database.select().from(segment);
    const changes = await handle.database.log.read({ tables: [segment], after: 0 });
    const read: Uint8Array[] = [];
    for await (const chunk of handle.blobs!.read(stored!.blob)) {
        read.push(chunk);
    }
    expect([
        stored!.blob,
        changes.changes.map((change) => change.scope),
        new TextDecoder().decode(Buffer.concat(read)),
    ]).toEqual([createHash("sha256").update("first").digest("hex"), [record.scope], "first"]);
});

test("copy a bucket to another host: follow its catalogue and blobs, then capture the rest once it stops writing", async () => {
    // keep a bucket with one file on the source host, and provision it on the target host
    const [sources, targets] = await Promise.all([temporary(), temporary()]);
    await using sourceHost = new LocalBucketHost({
        directory: sources,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    await using targetHost = new LocalBucketHost({
        directory: targets,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: CREDENTIALS,
    });
    const [source, target] = [localBucketProvider(sourceHost), localBucketProvider(targetHost)];
    const bucketId = identifier("bucket").parse(record.id);
    const written = await sourceHost.open({ bucketId }, record.scope);
    await written.put("notes/a.txt", "first");
    const from = await source.open({ ...record, ...(await source.provision(record)) }, []);
    onTestFinished(() => from.close());
    const to = await target.open({ ...record, ...(await target.provision(record)) }, []);
    await to.migrate(replicaTables);

    // follow the catalogue, fetching each referenced blob, while the source writes another file
    const copy = ZoneTransfer.database(record.id, to.database);
    const feed = new Feed(from.database, copy.tables);
    const blobs = { store: to.blobs!, source: from.blobs! };
    const controller = new AbortController();
    const following = copy.follow(
        to.database,
        (after, signal) => feed.subscribe(copy.queries, after, signal),
        controller.signal,
        { blobs },
    );
    await written.put("notes/b.txt", "second");
    const signal = AbortSignal.timeout(5000);
    await Replica.reach(to.database, record.id, await from.database.log.position(), signal);
    controller.abort();
    await following;

    // capture the rest once the source stops, then own the copy and drop its bookkeeping
    await written.delete("notes/a.txt");
    for await (const _page of copy.apply(to.database, feed.capture(copy.captured, signal), {
        blobs,
    })) {
        // apply each captured page
    }
    await copy.promote(to.database);
    await to.migrate([]);
    await to.close();

    // read the source's files on the target
    const received = await targetHost.open({ bucketId });
    const listed = await received.list();
    expect([
        listed.files.map((file) => file.key),
        await (await received.get("notes/b.txt"))!.text(),
    ]).toEqual([["notes/b.txt"], "second"]);
});
