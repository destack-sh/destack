import { sql } from "../../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../../test/database.ts";
import { Change, eq, Transaction } from "../../index.ts";
import { changeTables, first, note, revision } from "./fixture.ts";
import { testSimulation } from "@destack/test/world";

/** The concurrent writers of the PostgreSQL numbering test: four per pool, filling both pools. */
const WRITERS = 8;

/** The commits each writer makes. */
const COMMITS = 10;

/** Open a migrated test database in a world of its own. */
async function open() {
    const simulation = testSimulation();
    const storage = await TestDatabase.create(changeTables, {
        environment: simulation.environment("test"),
    });
    onTestFinished(() => storage.close());
    await storage.database.migrate(changeTables);

    return { storage, simulation, database: storage.database };
}

test("order postgresql changes by commit, not by write", async () => {
    const { database, simulation } = await open();

    // write first in a transaction that commits last
    const released = Promise.withResolvers<void>();
    const written = Promise.withResolvers<void>();
    const late = database.transaction(async (transaction) => {
        await transaction.insert(note).values({ ...first, id: "late" });
        written.resolve();
        await released.promise;
    });
    await written.promise;
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "early" })
            .execute(),
    );

    // hide the uncommitted write, then order it after the earlier commit
    const before = await database.log.read({ tables: [note], after: 0 });
    expect(before.changes.map((change) => change.key)).toEqual([{ id: "early" }]);
    released.resolve();
    await simulation.drive(late);
    const after = await database.log.read({ tables: [note], after: before.sequence });
    expect(after.changes.map((change) => change.key)).toEqual([{ id: "late" }]);
});

test("commit both of two postgresql transactions that read a row together and append to it, running the later again", async () => {
    const { database, simulation } = await open();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());

    // read the row in both before either commits, and append to the title read
    const both = Promise.withResolvers<void>();
    let reads = 0;
    const append = (suffix: string) =>
        Transaction.run(database, async (transaction) => {
            const [row] = await transaction.read(note).findMany({ where: { id: "a" } });
            if (row === undefined) {
                throw new TypeError("the note is missing");
            }
            reads += 1;
            if (reads === 2) {
                both.resolve();
            }
            await both.promise;
            await transaction.write({
                table: note,
                before: row,
                after: { ...row, title: row.title + suffix },
            });
        });
    await simulation.drive(Promise.all([append("!"), append("?")]));

    // commit both appends in one of the two orders, the later one read again
    const [row] = await database.select({ title: note.title }).from(note).where(eq(note.id, "a"));
    expect([[{ title: "First!?" }], [{ title: "First?!" }]]).toContainEqual([row]);
});

test("run a postgresql update that lost to a concurrent commit again after it, its write last", async () => {
    const { database, simulation } = await open();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());

    // commit both of two concurrent updates, the one losing the race again after the other
    const released = Promise.withResolvers<void>();
    const read = Promise.withResolvers<void>();
    const late = database.transaction(async (transaction) => {
        await transaction.select().from(note).where(eq(note.id, "a"));
        read.resolve();
        await released.promise;
        await transaction.update(note).set({ title: "Late" }).where(eq(note.id, "a"));
    });
    await read.promise;
    await database.transaction((transaction) =>
        transaction.update(note).set({ title: "Early" }).where(eq(note.id, "a")).execute(),
    );
    released.resolve();
    await simulation.drive(late);

    // keep the later update's title, written after the earlier commit
    expect(await database.select({ title: note.title }).from(note).where(eq(note.id, "a"))).toEqual(
        [{ title: "Late" }],
    );
});

test("number concurrent postgresql commits of two pools without gaps, each key's changes in commit order, at once for their writer", async () => {
    const { storage, simulation } = await open();
    const other = await storage.connect(changeTables);
    await storage.database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, editedAt: 0 })
            .execute(),
    );

    // follow every change from the start through the other pool
    const total = 1 + WRITERS * COMMITS * 2;
    const followed: Change[] = [];
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    const following = (async () => {
        const selection = { tables: [note, revision], after: 0 };
        for await (const page of other.log.follow(selection, stopping.signal)) {
            followed.push(...page.changes);
            if (followed.length >= total) {
                return;
            }
        }
    })();

    // commit from both pools at once, each counting the shared note up with a revision
    const committing = Promise.all(
        Array.from({ length: WRITERS }, async (_, writer) => {
            const database = writer % 2 === 0 ? storage.database : other;
            for (let number = 0; number < COMMITS; number++) {
                await database.transaction(
                    async (transaction) => {
                        await transaction
                            .update(note)
                            .set({ editedAt: sql`${note.editedAt} + 1` })
                            .where(eq(note.id, "a"));
                        await transaction.insert(revision).values({
                            scope: "inbox",
                            noteId: `writer-${writer}`,
                            number,
                            title: "Counted",
                        });
                    },
                    { isolationLevel: "read committed" },
                );
            }
        }),
    );
    await simulation.drive(committing);

    // read the last commit at once through its writer's pool
    expect((await storage.database.log.position()).sequence).toBe(total);

    // number every change once, without gaps
    await simulation.drive(following);
    expect(followed.map((change) => change.sequence)).toEqual(
        Array.from({ length: total }, (_, index) => index + 1),
    );

    // place each later update of the shared note at a higher position, with its higher count
    const counts = followed.flatMap((change) =>
        Change.of(change, note) ? [Change.after(change)?.editedAt] : [],
    );
    expect(counts).toEqual(Array.from({ length: WRITERS * COMMITS + 1 }, (_, index) => index));
});
