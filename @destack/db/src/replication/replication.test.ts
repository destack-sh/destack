import { expect, onTestFinished, test } from "@destack/test";
import type { Chunk } from "@destack/resource";
import { TestDatabase } from "../test/database.ts";
import { defineTable, TABLE, type Table } from "../table/table.ts";
import { binary, boolean, integer, json, text } from "../table/column.ts";
import { eq } from "../query/predicate.ts";
import { asc } from "../query/builder.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { schema } from "@destack/schema";
import { declareState } from "../migration/state.ts";
import { Replication } from "./replication.ts";

/** Nested folders. */
const folder = defineTable(
    "replication_folder",
    {
        /** The folder's key. */
        id: text("id").primaryKey(),
        /** The space the folder lives in. */
        scope: text("scope").notNull(),
        /** The containing folder, absent at the top. */
        parentId: text("parent_id").references((): never => folder.id as never, {
            onDelete: "cascade",
        }),
        /** Whether the folder is shared. */
        isShared: boolean("is_shared").notNull(),
    },
    { log: {} },
);

/** Notes in folders. */
const note = defineTable(
    "replication_note",
    {
        /** The note's key. */
        id: text("id").primaryKey(),
        /** The space the note lives in. */
        scope: text("scope").notNull(),
        /** The folder holding the note. */
        folderId: text("folder_id")
            .notNull()
            .references(() => folder.id, { onDelete: "cascade" }),
        /** The note's rank. */
        rank: integer("rank").notNull(),
        /** Its tags. */
        tags: json("tags", schema.array(schema.string())).notNull(),
    },
    { log: {} },
);

/** Files attached to notes, with bytes left out of the log. */
const attachment = defineTable(
    "replication_attachment",
    {
        /** The attachment's key. */
        id: text("id").primaryKey(),
        /** The space the attachment lives in. */
        scope: text("scope").notNull(),
        /** The note it is attached to. */
        noteId: text("note_id")
            .notNull()
            .references(() => note.id, { onDelete: "cascade" }),
        /** The file's bytes. */
        bytes: binary("bytes").notNull(),
    },
    { log: {} },
);

/** Unlogged drafts. */
const draft = defineTable("replication_draft", {
    /** The draft's key. */
    id: text("id").primaryKey(),
    /** The draft's text. */
    body: text("body").notNull(),
});

/** Every declared table, unordered. */
const TABLES = [attachment, draft, note, folder];

/** The desired state of the tables' database. */
const DESIRED = [
    {
        tables: {
            sqlite: declareState(TABLES, "sqlite"),
            postgresql: declareState(TABLES, "postgresql"),
        },
    },
];

/** Read each table's rows in key order. */
async function read(database: DatabaseConnection): Promise<Record<string, unknown[]>> {
    const read: Record<string, unknown[]> = {};
    for (const table of TABLES as Table[]) {
        const columns = table[TABLE].columns;
        read[table[TABLE].sqlName] = await database.select().from(table).orderBy(asc(columns.id!));
    }

    return read;
}

/** Export a stage from a cursor into the target, returning the last cursor. */
async function pass(
    source: TestDatabase,
    target: TestDatabase,
    stage: "live" | "fenced",
    after: string | undefined,
): Promise<string | undefined> {
    // connect each side with its dialect's tables
    const [exported, imported] = [source, target].map((side) =>
        Replication.of(DESIRED, side.database.dialect),
    );
    const [from, into] = await Promise.all([
        source.connect(exported!.tables),
        target.connect(imported!.tables),
    ]);

    // read the stage's chunks and import them
    let cursor = after;
    try {
        const chunks: Chunk[] = [];
        for await (const chunk of exported!.export(
            from,
            "replication:notes",
            stage,
            after,
            AbortSignal.timeout(4000),
        )) {
            chunks.push(chunk);
        }
        for (const chunk of chunks) {
            await imported!.import(into, chunk);
            cursor = chunk.cursor;
        }
    } finally {
        await Promise.all([from.close(), into.close()]);
    }

    return cursor;
}

test("copy a SQLite database into PostgreSQL exactly: live rows, their changes, then the fenced rest", async () => {
    const [source, target] = await Promise.all([
        TestDatabase.create("sqlite", TABLES, { isMigrated: true }),
        TestDatabase.create("postgresql", TABLES, { isMigrated: true }),
    ]);
    onTestFinished(async () => {
        await Promise.all([source.close(), target.close()]);
    });
    const copy = Replication.of(DESIRED, "sqlite");
    const from = source.database;

    // order live tables parents first and fence the rest
    expect(
        [copy.live, copy.fenced].map((tables) => tables.map((table) => table[TABLE].sqlName)),
    ).toEqual(
        [
            [folder, note],
            [attachment, draft],
        ].map((tables) => tables.map((table) => table[TABLE].sqlName)),
    );

    // write folders, notes, an attachment and a draft
    await from.insert(folder).values([
        { id: "f2", scope: "s", parentId: null, isShared: false },
        { id: "f1", scope: "s", parentId: "f2", isShared: true },
    ]);
    await from.insert(note).values([
        { id: "n1", scope: "s", folderId: "f1", rank: 1, tags: ["a"] },
        { id: "n2", scope: "s", folderId: "f2", rank: 2, tags: [] },
    ]);
    await from
        .insert(attachment)
        .values({ id: "a1", scope: "s", noteId: "n1", bytes: new Uint8Array([1, 2]) });
    await from.insert(draft).values({ id: "d1", body: "first" });

    // copy the live rows, then their changes
    let cursor = await pass(source, target, "live", undefined);
    await from
        .update(note)
        .set({ rank: 5, tags: ["b", "c"] })
        .where(eq(note.id, "n1"));
    await from.delete(note).where(eq(note.id, "n2"));
    await from.insert(note).values({ id: "n3", scope: "s", folderId: "f2", rank: 3, tags: [] });
    cursor = await pass(source, target, "live", cursor);

    // copy the fenced rest
    await from
        .update(attachment)
        .set({ bytes: new Uint8Array([3]) })
        .where(eq(attachment.id, "a1"));
    await from
        .insert(attachment)
        .values({ id: "a3", scope: "s", noteId: "n3", bytes: new Uint8Array([4]) });
    await from.update(draft).set({ body: "second" }).where(eq(draft.id, "d1"));
    await from.delete(folder).where(eq(folder.id, "f1"));
    await pass(source, target, "fenced", cursor);
    expect(await read(target.database)).toEqual(await read(from));

    // converge again from the start
    await target.database.insert(draft).values({ id: "d9", body: "stale" });
    await pass(source, target, "fenced", undefined);
    expect(await read(target.database)).toEqual(await read(from));
});

test("copy a tree larger than one chunk whose children sort before their parents, in bounded chunks", async () => {
    const [source, target] = await Promise.all([
        TestDatabase.create("sqlite", TABLES, { isMigrated: true }),
        TestDatabase.create("postgresql", TABLES, { isMigrated: true }),
    ]);
    onTestFinished(async () => {
        await Promise.all([source.close(), target.close()]);
    });

    // chain 1500 folders so a child's key sorts before its parent's
    const count = 1500;
    const id = (index: number) => `f${String(index).padStart(4, "0")}`;
    await source.database.insert(folder).values(
        Array.from({ length: count }, (_, index) => ({
            id: id(index),
            scope: "s",
            parentId: index === count - 1 ? null : id(index + 1),
            isShared: false,
        })),
    );

    // copy every stage and count each chunk's rows
    const [exported, imported] = [source, target].map((side) =>
        Replication.of(DESIRED, side.database.dialect),
    );
    const [from, into] = await Promise.all([
        source.connect(exported!.tables),
        target.connect(imported!.tables),
    ]);
    const sizes: number[] = [];
    for await (const chunk of exported!.export(
        from,
        "replication:tree",
        "fenced",
        undefined,
        AbortSignal.timeout(4000),
    )) {
        const body = chunk.body as { kind: string; rows?: unknown[]; links?: unknown[] };
        sizes.push((body.rows ?? body.links ?? []).length);
        await imported!.import(into, chunk);
    }
    expect([Math.max(...sizes) <= 1000, await read(target.database)]).toEqual([
        true,
        await read(source.database),
    ]);
});
