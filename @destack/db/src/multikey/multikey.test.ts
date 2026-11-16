import { expect, onTestFinished, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { schema } from "@destack/schema";
import { defineTable, eq, index, json, TABLE, text } from "../index.ts";
import { TestDatabase } from "../test/database.ts";

/** A playlist's items: tracks with a title and a length, and sections with a value. */
const ITEMS = schema.array(schema.json());

/** A playlist's owner and items, without an index. */
const plain = defineTable("playlist", {
    id: text("id").primaryKey(),
    owner: text("owner").notNull(),
    items: json("items", ITEMS).notNull(),
});

/** A playlist indexed by owner, track title and length, and sparsely by section. */
const playlist = defineTable(
    "playlist",
    {
        id: text("id").primaryKey(),
        owner: text("owner").notNull(),
        items: json("items", ITEMS).notNull(),
    },
    {
        constraints: (columns) => [
            index("playlist_track").on(
                columns.owner,
                columns.items.element("title", "text"),
                columns.items.element("seconds", "integer"),
            ),
            index("playlist_section")
                .on(
                    columns.items.element("section.kind", "text"),
                    columns.items.element("section.value", "json"),
                )
                .sparse(),
        ],
    },
);

/** The entry table of the playlist's tracks. */
const tracks = playlist[TABLE].entryTable("playlist_track");

/** The entry table of the playlist's sections. */
const sections = playlist[TABLE].entryTable("playlist_section");

test("keep one entry per array element through inserts, updates and deletes, with copied columns, nested paths and sparse indexes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([playlist], { environment: world, isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const read = async () => ({
        tracks: ordered(await database.select().from(tracks)),
        sections: ordered(await database.select().from(sections)),
    });

    // insert two playlists, one with a section, then rewrite the first and delete the second
    await database.insert(playlist).values([
        {
            id: "a",
            owner: "ada",
            items: [
                { title: "intro", seconds: 30 },
                { section: { kind: "chorus", value: { repeat: 2 } } },
            ],
        },
        { id: "b", owner: "bo", items: [{ title: "outro", seconds: 45 }] },
    ]);
    const inserted = await read();
    await database
        .update(playlist)
        .set({ owner: "cy", items: [{ title: "verse", seconds: 60 }] })
        .where(eq(playlist.id, "a"));
    await database.delete(playlist).where(eq(playlist.id, "b"));
    expect({ inserted, rewritten: await read() }).toEqual({
        inserted: {
            tracks: [
                { row: "a", position: 0, owner: "ada", title: "intro", seconds: 30 },
                { row: "a", position: 1, owner: "ada", title: null, seconds: null },
                { row: "b", position: 0, owner: "bo", title: "outro", seconds: 45 },
            ],
            sections: [
                { row: "a", position: 1, sectionKind: "chorus", sectionValue: { repeat: 2 } },
            ],
        },
        rewritten: {
            tracks: [{ row: "a", position: 0, owner: "cy", title: "verse", seconds: 60 }],
            sections: [],
        },
    });
});

test("fill a new index from the rows a table already has when a migration adds it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a playlist without the index, then migrate to the indexed table
    const storage = await TestDatabase.create([plain], { environment: world, isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database
        .insert(plain)
        .values([{ id: "a", owner: "ada", items: [{ title: "intro", seconds: 30 }] }]);
    await database.migrate([playlist]);
    expect(await database.select().from(tracks)).toEqual([
        { row: "a", position: 0, owner: "ada", title: "intro", seconds: 30 },
    ]);
});

/** Order entries by their row, then by their place in the row's array. */
function ordered(rows: readonly Record<string, unknown>[]) {
    return rows.toSorted((left, right) =>
        String(left["row"]) === String(right["row"])
            ? Number(left["position"]) - Number(right["position"])
            : String(left["row"]) < String(right["row"])
              ? -1
              : 1,
    );
}
