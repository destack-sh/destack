import { type Digest, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { MemoryContentStore } from "../test/store.ts";
import { defineTable, eq, text } from "../index.ts";
import { DatabaseHandle } from "./handle.ts";

/** Attachments with a title and the digest of their file. */
const attachment = defineTable("attachment", {
    /** The attachment's identifier. */
    id: text("id").primaryKey(),
    /** The space with the attachment. */
    scope: text("scope").notNull(),
    /** The attachment's title. */
    title: text("title").notNull(),
    /** The digest of the file's bytes. */
    file: text("file").notNull(),
});

/** The digest of the file most attachments share. */
const SHARED = "a".repeat(64);

/** Open two migrated SQLite databases of attachments, closed after the test. */
async function openPair() {
    const [source, target] = await Promise.all([
        TestDatabase.create("sqlite", [attachment], { isMigrated: true }),
        TestDatabase.create("sqlite", [attachment], { isMigrated: true }),
    ]);
    onTestFinished(async () => {
        await Promise.all([source.close(), target.close()]);
    });

    return [source, target] as const;
}

/** Read the digests of the pages a snapshot keeps. */
async function pagesOf(store: MemoryContentStore, snapshot: Digest): Promise<string[]> {
    const [bytes] = await Array.fromAsync(store.read(snapshot));
    const manifest = schema
        .object({ tables: schema.record(schema.string(), schema.array(schema.string())) })
        .parse(JSON.parse(new TextDecoder().decode(bytes)));

    return Object.values(manifest.tables).flat();
}

test("snapshot a database's rows into a content-addressed store and restore them into another, rewriting rows both ways", async () => {
    const [source, target] = await openPair();
    const store = new MemoryContentStore();
    await source.database
        .insert(attachment)
        .values({ id: "a", scope: "space", title: "notes", file: SHARED });

    // snapshot it with its title wrapped, and restore it unwrapped
    const digest = await DatabaseHandle.snapshot(source, store, async (_table, row) => ({
        ...row,
        title: `wrapped ${schema.string().parse(row["title"])}`,
    }));
    await DatabaseHandle.restore(target, digest, store, async (_table, row) => ({
        ...row,
        title: schema.string().parse(row["title"]).replace("wrapped ", ""),
    }));

    // keep the same rows in the target, under the same digest again
    expect([
        await target.database.select().from(attachment),
        await DatabaseHandle.snapshot(source, store),
    ]).toEqual([
        [{ id: "a", scope: "space", title: "notes", file: SHARED }],
        await DatabaseHandle.snapshot(target, store),
    ]);
});

test("restore onto a base snapshot only the pages an edit, an insert and a deletion changed", async () => {
    // keep 6001 attachments
    const [source, target] = await openPair();
    const store = new MemoryContentStore();
    const rows = Array.from({ length: 6001 }, (_, index) => ({
        id: `a${String(index).padStart(4, "0")}`,
        scope: "space",
        title: `attachment ${index}`,
        file: SHARED,
    }));
    for (let start = 0; start < rows.length; start += 500) {
        await source.database.insert(attachment).values(rows.slice(start, start + 500));
    }

    // restore a first snapshot, then edit, insert and delete one attachment each
    const base = await DatabaseHandle.snapshot(source, store);
    await DatabaseHandle.restore(target, base, store);
    await source.database
        .update(attachment)
        .set({ title: "renamed" })
        .where(eq(attachment.id, "a1500"));
    await source.database
        .insert(attachment)
        .values({ id: "a0500x", scope: "space", title: "inserted", file: SHARED });
    await source.database.delete(attachment).where(eq(attachment.id, "a6000"));

    // restore the second snapshot onto the first, reading only the manifests and the changed pages
    const digest = await DatabaseHandle.snapshot(source, store);
    const read: Digest[] = [];
    await DatabaseHandle.restore(
        target,
        digest,
        { read: (part) => (read.push(part), store.read(part)) },
        undefined,
        base,
    );
    const [before, after] = [await pagesOf(store, base), await pagesOf(store, digest)];
    const left = before.filter((page) => !after.includes(page));
    const arrived = after.filter((page) => !before.includes(page));
    expect({
        rows: await target.database.select().from(attachment).orderBy(attachment.id),
        pages: [before.length, left.length, arrived.length],
        read,
    }).toEqual({
        rows: await source.database.select().from(attachment).orderBy(attachment.id),
        pages: [6, 3, 3],
        read: [digest, base, ...left, ...arrived],
    });
});
