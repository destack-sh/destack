import { Digest } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { asc } from "../sql/index.ts";
import { TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { blob, blobs, integer, text } from "../table/column.ts";
import { Transaction } from "../log/commit.ts";
import { BLOB_TABLES, blobGarbage } from "./table.ts";

/** Documents whose body, renditions and expiring archive are blobs of the partition. */
const document = defineTable(
    "content_document",
    {
        /** The document. */
        id: text("id").primaryKey(),
        /** The folder the document lives in. */
        scope: text("scope").notNull(),
        /** The body's blob. */
        body: blob("body").notNull(),
        /** The renditions' blobs. */
        renditions: blobs("renditions").notNull(),
        /** The archive's blob, kept while the document is unexpired. */
        archive: blob("archive", { while: { expiredAt: { isNull: true } } }),
        /** When the document expired, absent while live. */
        expiredAt: integer("expired_at"),
    },
    { log: {} },
);

/** A digest of one repeated hex character, as a test stands for a blob. */
function digestOf(character: string): Digest {
    return Digest.parse(character.repeat(64));
}

test("retire the blobs a write may let go: written blob columns' old values, conditioned ones, deleted rows'", async () => {
    // keep a document with a body, two renditions and an archive
    const storage = await TestDatabase.create([document, ...BLOB_TABLES], {
        environment: testSimulation().environment("test"),
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const live = {
        id: "a",
        scope: "inbox",
        body: digestOf("1"),
        renditions: [digestOf("2"), digestOf("3")],
        archive: digestOf("4"),
        expiredAt: null,
    };
    await Transaction.run(database, (transaction) =>
        transaction.write({ table: document, before: null, after: live }),
    );
    const created = await database.select().from(blobGarbage);

    // replace the body and drop a rendition, then expire it, then delete it
    const edited = { ...live, body: digestOf("5"), renditions: [digestOf("2")] };
    await Transaction.run(database, (transaction) =>
        transaction.write({ table: document, before: live, after: edited }),
    );
    const replaced = await database.select().from(blobGarbage).orderBy(asc(blobGarbage.digest));
    await database.delete(blobGarbage);
    const expired = { ...edited, expiredAt: 1 };
    await Transaction.run(database, (transaction) =>
        transaction.write({ table: document, before: edited, after: expired }),
    );
    const flipped = await database.select().from(blobGarbage).orderBy(asc(blobGarbage.digest));
    await database.delete(blobGarbage);
    await Transaction.run(database, (transaction) =>
        transaction.write({ table: document, before: expired, after: null }),
    );
    const deleted = await database.select().from(blobGarbage).orderBy(asc(blobGarbage.digest));
    // refuse a raw deletion outside a transaction, which could lose its retirements
    const refused = await refusal(() => database.delete(document).execute());

    // retire what a collection then checks for references, kept ones included
    expect({ created, replaced, flipped, deleted, refused }).toEqual({
        created: [],
        replaced: [digestOf("1"), digestOf("2"), digestOf("3"), digestOf("4")].map((digest) => ({
            digest,
        })),
        flipped: [{ digest: digestOf("4") }],
        deleted: [digestOf("2"), digestOf("4"), digestOf("5")].map((digest) => ({ digest })),
        refused: [
            "BAD_REQUEST",
            "QUERY_INVALID",
            "a delete of content_document lets blobs go, so it runs in a transaction",
        ],
    });
});
