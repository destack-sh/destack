import { TEST_START, testSimulation, testWorldOptions } from "@destack/test/world";
import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { expect, testOn } from "@destack/test";
import { text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { asc } from "../sql/index.ts";
import { connectBunSqlite } from "./connection.ts";

/** Test against SQLite alone, as the file tests its own connector or host. */
const onSqlite = testOn("sqlite");

/** Notes by name, which both worlds write. */
const note = defineTable("note", {
    name: text("name").primaryKey(),
    world: text("world").notNull(),
});

onSqlite(
    "serialize two worlds' writes to one file in the process, the second once the first commits",
    async () => {
        // open one file from two worlds, each on its own clock
        await using first = testSimulation();
        await using second = new Simulation({
            world: testWorldOptions(),
            seed: 2,
            start: TEST_START,
        });
        const { files } = first.environment("first");
        const directory = await files.makeTemporaryDirectory("destack-sqlite-worlds-");
        const path = join(directory, "database.db");
        const holding = await connectBunSqlite(path, [note], {
            environment: first.environment("first"),
        });
        await holding.migrate([note]);
        const waiting = await connectBunSqlite(path, [note], {
            environment: second.environment("second"),
        });

        // hold a write transaction in the first world while the second world writes the file
        const held = Promise.withResolvers<void>();
        const transaction = holding.transaction(async (writing) => {
            await writing.insert(note).values({ name: "first", world: "first" }).execute();
            await held.promise;
        });
        const written = waiting.insert(note).values({ name: "second", world: "second" }).execute();
        held.resolve();
        await Promise.all([transaction, written]);

        // read both rows, the second written once the first world's transaction committed
        const rows = await waiting.select().from(note).orderBy(asc(note.name));
        await waiting.close();
        await holding.close();
        await files.remove(directory, { recursive: true });

        expect(rows).toEqual([
            { name: "first", world: "first" },
            { name: "second", world: "second" },
        ]);
    },
);
