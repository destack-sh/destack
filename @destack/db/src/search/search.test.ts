import { expect, onTestFinished, test } from "@destack/test";
import { asc, Condition, defineTable, eq, TABLE, text } from "../index.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { SearchText } from "./text.ts";

/** Notes before their text is searched. */
const plainNote = defineTable(
    "note",
    {
        /** The note. */
        id: text("id").primaryKey(),
        /** The scope. */
        scope: text("scope").notNull(),
        /** The title. */
        title: text("title").notNull(),
        /** The body, absent for a title alone. */
        body: text("body"),
    },
    { log: {} },
);

/** Notes whose title and body are searched. */
const note = defineTable(
    "note",
    {
        /** The note. */
        id: text("id").primaryKey(),
        /** The scope. */
        scope: text("scope").notNull(),
        /** The title. */
        title: text("title").notNull(),
        /** The body, absent for a title alone. */
        body: text("body"),
    },
    { log: {}, fullText: ["title", "body"] },
);

/** The searches each test runs, alike in SQL and memory. */
const SEARCHES = [
    "ada",
    "ada lov",
    "LOVELACE",
    "café",
    "東京",
    "engine-note",
    "eng not",
    "zzz",
    "char",
];

test.for(TEST_DIALECTS)(
    "search %s rows by word prefixes, kept through writes and a migration, alike in SQL and memory",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [plainNote], { storage: "file" });
        onTestFinished(() => storage.close());

        // keep notes before their text is searched, then search it over a connection declaring the search
        await storage.database.migrate([plainNote]);
        await storage.database.insert(plainNote).values([
            { id: "a", scope: "s", title: "Ada Lovelace", body: "The first program" },
            { id: "b", scope: "s", title: "Analytical Engine", body: "engine-note by Ada" },
        ]);
        const database = await storage.connect([note]);
        onTestFinished(() => database.close());
        await database.migrate([note]);

        // add, change and remove notes once their text is searched
        await database.insert(note).values([
            { id: "c", scope: "s", title: "Café in 東京", body: null },
            { id: "d", scope: "s", title: "Removed", body: "Ada" },
        ]);
        await database.update(note).set({ body: "Notes on Lovelace" }).where(eq(note.id, "b"));
        await database.transaction(async (transaction) => {
            await transaction.delete(note).where(eq(note.id, "d"));
            await transaction.update(note).set({ title: "Charles" }).where(eq(note.id, "a"));
        });

        // search in SQL and in memory, each search finding the same notes
        const rows = await database.select().from(note).orderBy(asc(note.id));
        const found = async (search: string) => {
            const selected = await database
                .select({ id: note.id })
                .from(note)
                .where(Condition.render({ SEARCH: search }, note))
                .orderBy(asc(note.id));
            const matched = rows.filter((row) =>
                note[TABLE].requireFullText().matches(row, SearchText.query(search)),
            );

            return [selected.map((row) => row.id), matched.map((row) => row.id)];
        };
        expect(
            Object.fromEntries(
                await Promise.all(SEARCHES.map(async (search) => [search, await found(search)])),
            ),
        ).toEqual({
            ada: [[], []],
            "ada lov": [[], []],
            LOVELACE: [["b"], ["b"]],
            café: [["c"], ["c"]],
            東京: [["c"], ["c"]],
            "engine-note": [["b"], ["b"]],
            "eng not": [["b"], ["b"]],
            zzz: [[], []],
            char: [["a"], ["a"]],
        });

        // refuse a search of no words
        expect(() => Condition.render({ SEARCH: " - " }, note)).toThrow(
            "a search takes 1 to 32 words, not 0:  - ",
        );
    },
);
