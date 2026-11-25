import { expect, onTestFinished, test } from "@destack/test";
import { asc, Condition, defineTable, eq, Expression, Order, sql, TABLE, text } from "../index.ts";
import { declareState } from "../migration/state.ts";
import { changedTables, planTables } from "../migration/plan.ts";
import { TestDatabase } from "../test/database.ts";
import { Search } from "./search.ts";
import { schema } from "@destack/schema";
import { testSimulation } from "@destack/test/world";

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
    "cafe",
    "istanbul",
    "İST",
    "東京",
    "engine-note",
    "eng not",
    "zzz",
    "char",
];

test("search rows by word prefixes, current through writes and a migration, alike in SQL and memory", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([plainNote], {
        environment: world,
        storage: "file",
    });
    onTestFinished(() => storage.close());

    // store notes before their text is searched, then search it over a connection with search
    await storage.database.migrate([plainNote]);
    await storage.database.transaction((transaction) =>
        transaction
            .insert(plainNote)
            .values([
                { id: "a", scope: "s", title: "Ada Lovelace", body: "The first program" },
                { id: "b", scope: "s", title: "Analytical Engine", body: "engine-note by Ada" },
            ])
            .execute(),
    );
    const database = await storage.connect([note]);
    onTestFinished(() => database.close());
    await database.migrate([note]);

    // add, change and remove notes once their text is searched
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values([
                { id: "c", scope: "s", title: "Café in 東京", body: "İstanbul" },
                { id: "d", scope: "s", title: "Removed", body: "Ada" },
            ])
            .execute(),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(note)
                .set({ body: "Notes on Lovelace" })
                .where(eq(note.id, "b")),
    );
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
            note[TABLE].requireFullText().matches(row, Search.parse(search)),
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
        cafe: [["c"], ["c"]],
        istanbul: [["c"], ["c"]],
        İST: [["c"], ["c"]],
        東京: [["c"], ["c"]],
        "engine-note": [["b"], ["b"]],
        "eng not": [["b"], ["b"]],
        zzz: [[], []],
        char: [["a"], ["a"]],
    });

    // score titles in SQL and in memory alike, ordering the notes by it
    const scored = async (search: string) => {
        const relevance = Expression.relevance(["title", "body"], search);
        const selected = await database
            .select({ id: note.id, score: Expression.select(relevance, note) })
            .from(note)
            .orderBy(
                ...Order.render(Order.of({ relevance: "desc", id: "asc" }), note, {
                    extras: { relevance },
                }),
            );
        const memory = new Map(
            rows.map((row) => [row.id, Expression.evaluate(relevance, row)] as const),
        );

        return selected.map(({ id, score }) => [id, score, memory.get(id)]);
    };
    expect(
        Object.fromEntries(
            await Promise.all(
                [
                    "charles",
                    "cha",
                    "analytical eng",
                    "engine analytical",
                    "eng anal",
                    "lovelace",
                ].map(async (search) => [search, await scored(search)] as const),
            ),
        ),
    ).toEqual({
        charles: scoresOf([["a", 10]]),
        cha: scoresOf([["a", 8]]),
        "analytical eng": scoresOf([["b", 8]]),
        "engine analytical": scoresOf([["b", 6]]),
        "eng anal": scoresOf([["b", 4]]),
        lovelace: scoresOf([["b", 5]]),
    });

    // list the words that start alike, with their rows
    const terms = await note[TABLE].requireFullText().terms(database, "ancient");
    expect(terms.toSorted((one, other) => (one.term < other.term ? -1 : 1))).toEqual([
        { term: "analytical", rows: 1 },
    ]);

    // refuse a search of no words
    expect(() => Condition.render({ SEARCH: " - " }, note)).toThrow(
        "search ' - ' is invalid: it has 0 words, not 1 to 32",
    );
});

/** Expect every note's score in SQL and in memory, the best first, 2 for one not given. */
function scoresOf(given: readonly (readonly [id: string, score: number])[]) {
    const scores = new Map(given);

    return ["a", "b", "c"]
        .map((id) => [id, scores.get(id) ?? 2, scores.get(id) ?? 2] as const)
        .toSorted(([, one], [, other]) => other - one);
}

/** Letters of every script, with what search folds, cuts or leaves out. */
const LETTERS = Array.from(
    [
        "abcDEF12 -'.",
        "éÉñÅøŁđÆœßþıİς",
        "ЙйёЁъ",
        "ΟΔΣσςάώ",
        "ﬁＡＢ²½١٢۳०९",
        "مُحَمَّدأإ",
        "שָׁלוֹם",
        "हिन्दी",
        "สวัสดีครับ",
        "東京都庁買",
        "ガカラスを",
        "한국어",
        "\u0301\u0308\u200d\u00ad",
        "😀",
        "𠮷",
        "𝐁",
    ].join(""),
);

test("match and score random text of every script alike in SQL and memory", async () => {
    // store random titles, and search for pieces of them
    const environment = testSimulation().environment("test");
    const { random } = environment;
    const below = (limit: number) => {
        const [high = 0, low = 0] = random.bytes(2);

        return (high * 256 + low) % limit;
    };
    const randomText = (length: number) =>
        Array.from({ length: 1 + below(length) }, () => LETTERS[below(LETTERS.length)]).join("");
    const storage = await TestDatabase.create([note], { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const titles = Array.from({ length: 120 }, () => randomText(9));
    await storage.database.transaction((transaction) =>
        transaction
            .insert(note)
            .values(
                titles.map((title, index) => ({
                    id: `n${String(index).padStart(3, "0")}`,
                    scope: "s",
                    title,
                    body: randomText(6),
                })),
            )
            .execute(),
    );
    const rows = await storage.database.select().from(note).orderBy(asc(note.id));
    const searches = titles.slice(0, 60).flatMap((title) => {
        const characters = Array.from(title);
        const piece = characters.slice(0, 1 + below(3)).join("");

        return [title, piece].filter((search) => !Search.of(search).isEmpty);
    });

    // compare each search's matches and scores
    const differences: unknown[] = [];
    for (const typed of searches) {
        const search = Search.parse(typed);
        const relevance = Expression.relevance(["title", "body"], typed);
        const selected = await storage.database
            .select({ id: note.id, score: Expression.select(relevance, note) })
            .from(note)
            .where(Condition.render({ SEARCH: typed }, note));
        const scores = new Map(selected.map(({ id, score }) => [id, score]));
        for (const row of rows) {
            const isMatch = note[TABLE].requireFullText().matches(row, search);
            const score = isMatch ? Expression.evaluate(relevance, row) : undefined;
            if (scores.get(row.id) !== score) {
                differences.push([typed, row.title, row.body, scores.get(row.id), score]);
            }
        }
    }

    expect(differences).toEqual([]);
});

test("look rows up by their keys, whether searched or changed", async () => {
    // index more rows than one statement takes, then change them all
    const environment = testSimulation().environment("test");
    const storage = await TestDatabase.create([note], { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const ids = Array.from({ length: 1200 }, (_, index) => `N-${String(index)}`);
    await storage.database.transaction((transaction) =>
        transaction
            .insert(note)
            .values(ids.map((id) => ({ id, scope: "s", title: `First ${id}`, body: null })))
            .execute(),
    );
    await storage.database.transaction(
        async (transaction) => await transaction.update(note).set({ title: "Second" }),
    );
    const count = async (typed: string) =>
        (
            await storage.database
                .select({ id: note.id })
                .from(note)
                .where(Condition.render({ SEARCH: typed }, note))
        ).length;
    expect([await count("first"), await count("second")]).toEqual([0, 1200]);

    // scan no table in SQLite
    if (storage.database.dialect === "sqlite") {
        const where = Condition.render({ SEARCH: "second" }, note);
        const steps = await storage.database.transaction((transaction) =>
            transaction.execute(
                sql`EXPLAIN QUERY PLAN SELECT id FROM ${note} WHERE ${where}`,
                schema.looseObject({ detail: schema.string() }),
            ),
        );
        expect(steps[0]?.detail).toBe(
            "SEARCH destack__db__note USING COVERING INDEX sqlite_autoindex_destack__db__note_1 (id=?)",
        );
    }
});

test("rebuild an index the database cut", () => {
    // plan the same table over one migrated before the index folded accents
    const declared = declareState([note], "sqlite");
    const steps = (applied: typeof declared) =>
        planTables({
            applied,
            existing: [],
            declared,
            dialect: "sqlite",
            changed: changedTables(applied, declared),
            empty: new Set(),
        }).steps.flatMap((step) => step.statements);
    const kept = declared.map(({ fullTextVersion: _version, ...state }) => state);

    expect({ same: steps(declared), kept: steps(kept) }).toEqual({
        same: [],
        kept: [
            'DROP TABLE IF EXISTS "destack__db__note__search_terms"',
            'DROP TABLE IF EXISTS "destack__db__note__search"',
            `CREATE VIRTUAL TABLE "destack__db__note__search" USING fts5("key" UNINDEXED, "row", "text0", "text1", tokenize = 'ascii')`,
            `CREATE VIRTUAL TABLE "destack__db__note__search_terms" USING fts5vocab("destack__db__note__search", 'row')`,
        ],
    });
});
