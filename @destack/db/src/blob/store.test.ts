import { Simulation } from "@destack/environment/simulation";
import { blob, blobMap, blobs, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { eq } from "../sql/operator.ts";
import { TestDatabase } from "../test/index.ts";
import { BLOB_TABLES } from "./table.ts";
import { MemoryBlobStorage } from "@destack/resource";
import { Digest } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { BlobStore } from "./store.ts";

/** When every blob store test starts: 1 October 2026, UTC. */
const START = Date.UTC(2026, 9, 1);

/** How long the store leases a write: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** Attachments whose file and renditions are blobs of the partition. */
const attachment = defineTable("attachment", {
    /** The attachment. */
    id: text("id").primaryKey(),
    /** The file's blob. */
    file: blob("file").notNull(),
    /** The renditions' blobs, such as thumbnails. */
    renditions: blobs("renditions").notNull().default([]),
    /** The captions' blobs by language. */
    captions: blobMap("captions").notNull().default({}),
});

/** Bundles of blobs, each record describing its bundle's own blob and going with it. */
const bundle = defineTable("bundle", {
    /** The bundle's blob. */
    digest: blob("digest", { onDelete: "cascade" }).primaryKey(),
});

/** The blobs each bundle lists: files, or other bundles. */
const member = defineTable("member", {
    /** The listing bundle, whose deletion takes the row. */
    bundle: blob("bundle", { onDelete: "cascade" }).primaryKey(),
    /** The listed blob, kept while the row lasts. */
    digest: blob("digest").primaryKey(),
});

/** Open a blob store over memory storage and a test database with attachments and bundles. */
async function openBlobs() {
    const simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("bucket");
    const opened = await TestDatabase.create([attachment, bundle, member, ...BLOB_TABLES], {
        environment,
        isMigrated: true,
    });
    onTestFinished(() => opened.close());
    const storage = new MemoryBlobStorage(environment.crypto);
    const store = new BlobStore({ storage, database: opened.database, environment });

    return { simulation, environment, store, storage, database: opened.database };
}

/** Put some text as a blob of a store, answering its digest. */
async function put(store: BlobStore, content: string, crypto: Parameters<typeof Digest.of>[1]) {
    const bytes = new TextEncoder().encode(content);
    const digest = await Digest.of(bytes, crypto);
    await store.put(digest, async () =>
        (async function* () {
            yield bytes;
        })(),
    );

    return digest;
}

test("keep a blob while a row references it or its write's lease lasts, and delete it once neither does", async () => {
    // put a referenced blob and an unreferenced one
    const { simulation, environment, store, storage, database } = await openBlobs();
    const kept = await put(store, "kept", environment.crypto);
    const dropped = await put(store, "dropped", environment.crypto);
    await database.transaction(async (transaction) => {
        await transaction.insert(attachment).values({ id: "a", file: kept });
    });

    // collect within the leases, after them, and after the row's deletion retires its blob
    const within = await store.collect();
    simulation.clock.advance(LEASE_MILLISECONDS);
    const after = await store.collect();
    const remaining = [...(await storage.missing([kept, dropped]))];
    await database.transaction(async (transaction) => {
        await transaction.delete(attachment).where(eq(attachment.id, "a"));
    });
    const retired = await store.collect();
    expect({
        within: within === undefined ? undefined : within - START,
        after,
        missing: remaining,
        retired,
        left: [...(await storage.missing([kept, dropped]))],
    }).toEqual({
        within: LEASE_MILLISECONDS,
        after: undefined,
        missing: [dropped],
        retired: undefined,
        left: [kept, dropped],
    });
});

test("rescue a blob a deleted row let go when a writer leases it again before the collection", async () => {
    // let a blob go with its row, then lease it again for a new row before collecting
    const { simulation, environment, store, storage, database } = await openBlobs();
    const digest = await put(store, "shared", environment.crypto);
    simulation.clock.advance(LEASE_MILLISECONDS);
    await database.transaction(async (transaction) => {
        await transaction.insert(attachment).values({ id: "a", file: digest });
    });
    await database.transaction(async (transaction) => {
        await transaction.delete(attachment).where(eq(attachment.id, "a"));
    });
    await store.put(digest, () => Promise.reject(new TypeError("the blob is stored")));
    await database.transaction(async (transaction) => {
        await transaction.insert(attachment).values({ id: "b", file: digest });
    });

    // keep it through a collection and past the new lease
    await store.collect();
    simulation.clock.advance(LEASE_MILLISECONDS);
    await store.collect();
    expect(await storage.missing([digest])).toEqual([]);
});

test("keep the blobs a JSON array column lists, and delete one once no row lists it", async () => {
    // list two renditions on a row, past their writes' leases
    const { simulation, environment, store, storage, database } = await openBlobs();
    const file = await put(store, "file", environment.crypto);
    const small = await put(store, "small", environment.crypto);
    const large = await put(store, "large", environment.crypto);
    await database.transaction(async (transaction) => {
        await transaction.insert(attachment).values({ id: "c", file, renditions: [small, large] });
    });
    simulation.clock.advance(LEASE_MILLISECONDS);
    await store.collect();
    const kept = [...(await storage.missing([file, small, large]))];

    // drop the large rendition from the row, and collect
    await database.transaction(async (transaction) => {
        await transaction
            .update(attachment)
            .set({ renditions: [small] })
            .where(eq(attachment.id, "c"));
    });
    await store.collect();
    expect({ kept, missing: [...(await storage.missing([file, small, large]))] }).toEqual({
        kept: [],
        missing: [large],
    });
});

test("keep the blobs a map column lists by key, and delete one once no row lists it", async () => {
    // list two captions on a row, past their writes' leases
    const { simulation, environment, store, storage, database } = await openBlobs();
    const file = await put(store, "file", environment.crypto);
    const english = await put(store, "english", environment.crypto);
    const german = await put(store, "german", environment.crypto);
    await database.transaction(async (transaction) => {
        await transaction
            .insert(attachment)
            .values({ id: "c", file, captions: { en: english, de: german } });
    });
    simulation.clock.advance(LEASE_MILLISECONDS);
    await store.collect();
    const kept = [...(await storage.missing([file, english, german]))];

    // drop the German caption from the row, and collect
    await database.transaction(async (transaction) => {
        await transaction
            .update(attachment)
            .set({ captions: { en: english } })
            .where(eq(attachment.id, "c"));
    });
    await store.collect();
    expect({ kept, missing: [...(await storage.missing([file, english, german]))] }).toEqual({
        kept: [],
        missing: [german],
    });
});

test("delete a bundle no row keeps with its records, then the bundles and files only it listed, in one collection", async () => {
    // record an outer bundle listing a file and an inner bundle, which lists itself and a file an attachment shares
    const { simulation, environment, store, storage, database } = await openBlobs();
    const [outer, inner, own, shared] = [
        await put(store, "outer", environment.crypto),
        await put(store, "inner", environment.crypto),
        await put(store, "own", environment.crypto),
        await put(store, "shared", environment.crypto),
    ];
    await database.transaction(async (transaction) => {
        await transaction.insert(bundle).values([{ digest: outer }, { digest: inner }]);
        await transaction.insert(member).values([
            { bundle: outer, digest: own },
            { bundle: outer, digest: inner },
            { bundle: inner, digest: shared },
            { bundle: inner, digest: inner },
        ]);
        await transaction.insert(attachment).values({ id: "a", file: shared });
    });

    // collect past the leases, where only the attachment and the bundles' own leases kept anything
    simulation.clock.advance(LEASE_MILLISECONDS);
    await store.collect();
    const digests = [outer, inner, own, shared];
    expect({
        missing: await storage.missing(digests),
        bundles: await database.select().from(bundle),
        members: await database.select().from(member),
    }).toEqual({ missing: [outer, inner, own], bundles: [], members: [] });
});
