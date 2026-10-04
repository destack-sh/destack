import { createHash } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Dialect, Table } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceId } from "@destack/resource";
import { type Identifier, present, schema } from "@destack/schema";
import { ZoneTransfer } from "@destack/space/server";
import { Feed, Replica, replicaTables } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { StorageError } from "../error/index.ts";
import { LocalBucket, LocalBucketHost } from "../local/index.ts";
import { bucketProvider } from "../provider/index.ts";
import { S3Location, S3Server, SignatureV4 } from "../s3/index.ts";
import { type CatalogueStore, R2BucketHost } from "./host.ts";

/** The bucket resource the hosts keep. */
const record = {
    id: ResourceId.parse("bucket-01996ab0-0000-7000-8000-000000000002"),
    scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
    spec: {},
    reference: null,
};

/** The bucket of the resource. */
const bucketId = schema.identifier("bucket").parse(record.id);

/** The credentials the hosts presign with. */
const CREDENTIALS = { accessKeyId: "CELL", secretAccessKey: "cell-secret" };

/** The cell's S3 endpoint. */
const ENDPOINT = new URL("https://s3.cell.test");

/** Hash text as a blob digest. */
function digest(text: string): string {
    return createHash("sha256").update(text).digest("hex");
}

/** Catalogues as test databases of one dialect, one per bucket, as a cell keeps one schema per bucket in its database. */
class TestCatalogueStore implements CatalogueStore, AsyncDisposable {
    /** The dialect of the databases. */
    readonly #dialect: Dialect;
    /** The database of each bucket. */
    readonly #databases = new Map<string, TestDatabase>();

    /** Keep catalogues in one dialect. */
    constructor(dialect: Dialect) {
        this.#dialect = dialect;
    }

    /** Report whether a bucket has a catalogue. */
    async has(id: Identifier<"bucket">): Promise<boolean> {
        return this.#databases.has(id);
    }

    /** Connect to a bucket's catalogue, creating an empty database for it once. */
    async connect(id: Identifier<"bucket">, tables: readonly Table[]) {
        let database = this.#databases.get(id);
        if (database === undefined) {
            database = await TestDatabase.create(this.#dialect, [], { storage: "file" });
            this.#databases.set(id, database);
        }

        return database.connect(tables);
    }

    /** Remove a bucket's catalogue. */
    async destroy(id: Identifier<"bucket">): Promise<void> {
        await this.#databases.get(id)?.close();
        this.#databases.delete(id);
    }

    /** Remove every catalogue. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const database of this.#databases.values()) {
            await database.close();
        }
    }
}

/** Open a temporary directory removed after the test. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-r2-host-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

test.for(TEST_DIALECTS)(
    "provision a bucket as a prefix of the residency's bucket beside its catalogue, serve presigned writes and reads, and destroy both on %s",
    async (dialect) => {
        // keep the residency's bucket locally, as R2 keeps it, and the catalogues in test databases
        await using files = await LocalBucket.open(join(await temporary(), "files"), "residency");
        await using catalogues = new TestCatalogueStore(dialect);
        await using buckets = new R2BucketHost({
            files,
            name: "destack-test-files-eu",
            catalogues,
            endpoint: ENDPOINT,
            region: "auto",
            credentials: CREDENTIALS,
        });
        const provider = bucketProvider.r2(buckets);

        // refuse a bucket never provisioned, then provision it twice under one reference
        await expect(buckets.open({ bucketId })).rejects.toMatchObject({
            code: "NO_SUCH_BUCKET",
            message: `no bucket ${bucketId}`,
        });
        const provision = await provider.provision.provision(record);
        expect(await provider.provision.provision(record)).toEqual(provision);

        // write a file through a URL presigned at the located endpoint, and read it back the same way
        const server = new S3Server({
            region: "auto",
            credentials: async (id) => (id === CREDENTIALS.accessKeyId ? CREDENTIALS : undefined),
            open: (name) => buckets.named(name),
        });
        const { location } = await buckets.locate({ scope: record.scope, bucketId }, "write");
        const signer = new SignatureV4({ region: location.region });
        const url = S3Location.url(location, "notes/a.txt").href;
        const write = await signer.presign(
            new Request(url, { method: "PUT" }),
            CREDENTIALS,
            60,
            Date.now(),
        );
        const written = await server.fetch(
            new Request(write.url, { method: "PUT", body: "first" }),
        );
        const read = await server.fetch(
            await signer.presign(new Request(url), CREDENTIALS, 60, Date.now()),
        );

        // keep the blob below the bucket's prefix, then replace the file and collect the old blob
        const listed = async () => (await files.list()).files.map((file) => file.key);
        const before = await listed();
        const bucket = await buckets.open({ bucketId });
        await bucket.put("notes/a.txt", "second");
        await bucket.collect();
        const after = await listed();

        // destroy the bucket's blobs and catalogue
        await provider.provision.destroy({ ...record, ...provision });
        expect({
            provision,
            written: written.status,
            read: [read.status, await read.text()],
            before,
            after,
            destroyed: [
                await listed(),
                await catalogues.has(bucketId),
                await buckets.named(bucketId),
            ],
        }).toEqual({
            provision: { reference: `r2://destack-test-files-eu/${bucketId}/` },
            written: 200,
            read: [200, "first"],
            before: [`${bucketId}/${digest("first")}`],
            after: [`${bucketId}/${digest("second")}`],
            destroyed: [[], false, undefined],
        });
    },
);

test.for(TEST_DIALECTS)(
    "copy a bucket from a device host to a cell: follow its catalogue and blobs, then capture the rest once it stops writing, retiring the deleted file's blob on %s",
    async (dialect) => {
        // keep a bucket with one file on the device host, and provision it on the cell
        await using device = new LocalBucketHost({
            directory: await temporary(),
            endpoint: new URL("http://s3.localhost"),
            region: "local",
            credentials: CREDENTIALS,
        });
        await using files = await LocalBucket.open(join(await temporary(), "files"), "residency");
        await using catalogues = new TestCatalogueStore(dialect);
        await using cell = new R2BucketHost({
            files,
            name: "destack-test-files-eu",
            catalogues,
            endpoint: ENDPOINT,
            region: "auto",
            credentials: CREDENTIALS,
        });
        const [source, target] = [bucketProvider.local(device), bucketProvider.r2(cell)];
        const written = await device.open({ bucketId }, record.scope);
        await written.put("notes/a.txt", "first");
        const from = await source.open.open(
            { ...record, ...(await source.provision.provision(record)) },
            [],
        );
        onTestFinished(() => from.close());
        const to = await target.open.open(
            { ...record, ...(await target.provision.provision(record)) },
            [],
        );
        await to.migrate(replicaTables);

        // follow the catalogue, fetching each referenced blob, while the device writes another file
        const copy = ZoneTransfer.copy(record.id, to.database);
        const feed = new Feed(from.database, copy.tables);
        const blobs = { store: present(to.blobs, "blobs"), source: present(from.blobs, "blobs") };
        const controller = new AbortController();
        const following = copy.follow(
            to.database,
            ({ after }, signal) => feed.subscribe(copy.queries, after, signal),
            controller.signal,
            { blobs },
        );
        await written.put("notes/b.txt", "second");
        const signal = AbortSignal.timeout(5000);
        await Replica.reach(to.database, record.id, await from.database.log.position(), signal);
        controller.abort();
        await following;

        // capture the rest once the device stops, then own the copy and drop its bookkeeping
        await written.delete("notes/a.txt");
        await Array.fromAsync(
            copy.apply(to.database, feed.capture(copy.captured, signal), { blobs }),
        );
        await copy.promote(to.database);
        await to.migrate([]);

        // read the device's files on the cell, only their blobs left in the residency's bucket
        const received = await cell.open({ bucketId });
        const listed = await received.list();
        expect([
            listed.files.map((file) => file.key),
            await present(await received.get("notes/b.txt"), "notes/b.txt").text(),
            (await files.list()).files.map((file) => file.key),
        ]).toEqual([["notes/b.txt"], "second", [`${bucketId}/${digest("second")}`]]);
    },
);

test.for(TEST_DIALECTS)(
    "fence a cell bucket through its provider: refuse presigned and direct writes across a restart, read on, and lift on %s",
    async (dialect) => {
        // keep a bucket with one file on the cell, served over S3
        await using files = await LocalBucket.open(join(await temporary(), "files"), "residency");
        await using catalogues = new TestCatalogueStore(dialect);
        const options = {
            files,
            name: "destack-test-files-eu",
            catalogues,
            endpoint: ENDPOINT,
            region: "auto",
            credentials: CREDENTIALS,
        };
        const buckets = new R2BucketHost(options);
        const provider = bucketProvider.r2(buckets);
        const provisioned = { ...record, ...(await provider.provision.provision(record)) };
        await (await buckets.open({ bucketId })).put("notes/a.txt", "first");
        const reference = { scope: record.scope, bucketId };
        const { location } = await buckets.locate(reference, "write");
        const server = new S3Server({
            region: "auto",
            credentials: async (id) => (id === CREDENTIALS.accessKeyId ? CREDENTIALS : undefined),
            open: (name) => buckets.named(name),
        });
        const signer = new SignatureV4({ region: location.region });
        const url = S3Location.url(location, "notes/b.txt").href;
        const presigned = await signer.presign(
            new Request(url, { method: "PUT" }),
            CREDENTIALS,
            60,
            Date.now(),
        );

        // fence the bucket, refusing the presigned write and a lease for another, while reads go on
        await provider.fence.fence(provisioned);
        const put = await server.fetch(
            new Request(presigned.url, { method: "PUT", body: "second" }),
        );
        const fenced = new StorageError("FENCED", "bucket is fenced while a transfer copies it");
        const leased = await buckets.locate(reference, "write").catch((error: unknown) => error);
        const read = await (await (await buckets.open({ bucketId })).get("notes/a.txt"))?.text();

        // keep refusing writes after a restart, until the fence lifts
        await buckets[Symbol.asyncDispose]();
        await using restarted = new R2BucketHost(options);
        const reopened = await restarted.open({ bucketId });
        const refused = await reopened
            .put("notes/b.txt", "second")
            .catch((error: unknown) => error);
        await bucketProvider.r2(restarted).fence.lift(provisioned);
        await reopened.put("notes/b.txt", "second");
        expect({
            put: [put.status, await put.text()],
            leased,
            read,
            refused,
            lifted: await (await reopened.get("notes/b.txt"))?.text(),
        }).toEqual({
            put: [
                503,
                `<?xml version="1.0" encoding="UTF-8"?><Error><Code>ServiceUnavailable</Code><Message>${fenced.message}</Message><Resource>/${bucketId}/notes/b.txt</Resource><RequestId>${put.headers.get("x-amz-request-id")}</RequestId></Error>`,
            ],
            leased: fenced,
            read: "first",
            refused: fenced,
            lifted: "second",
        });
    },
);

test.for(TEST_DIALECTS)(
    "list a cell bucket's files and groups in key byte order on %s",
    async (dialect) => {
        // keep keys whose byte order differs from a language's collation
        await using files = await LocalBucket.open(join(await temporary(), "files"), "residency");
        await using catalogues = new TestCatalogueStore(dialect);
        await using buckets = new R2BucketHost({
            files,
            name: "destack-test-files-eu",
            catalogues,
            endpoint: ENDPOINT,
            region: "auto",
            credentials: CREDENTIALS,
        });
        const bucket = await buckets.open({ bucketId }, record.scope);
        const keys = ["ab", "a_c", "aB", "a-b", "a/z", "Z", "é"];
        for (const key of keys) {
            await bucket.put(key, key);
        }

        // list every file, a page after a key, and the groups below a prefix
        const listed = await bucket.list();
        const after = await bucket.list({ startAfter: "a-b", limit: 2 });
        const grouped = await bucket.list({ prefix: "a", delimiter: "/" });
        expect({
            listed: listed.files.map((file) => file.key),
            after: after.files.map((file) => file.key),
            grouped: [grouped.files.map((file) => file.key), grouped.delimitedPrefixes],
        }).toEqual({
            listed: ["Z", "a-b", "a/z", "aB", "a_c", "ab", "é"],
            after: ["a/z", "aB"],
            grouped: [["a-b", "aB", "a_c", "ab"], ["a/"]],
        });
    },
);
