import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { DatabaseError } from "../error/error.ts";
import { asc, eq, type DatabaseConnection } from "../index.ts";
import { changeTables, note, revision } from "./test/fixture.ts";

test.for(TEST_DIALECTS)(
    "read every logged table as it was at a position, with its values' exact types, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, changeTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const { database } = storage;
        const notes = (connection: DatabaseConnection) =>
            connection
                .select({
                    id: note.id,
                    title: note.title,
                    summary: note.summary,
                    views: note.views,
                    labels: note.labels,
                    editedAt: note.editedAt,
                })
                .from(note)
                .orderBy(asc(note.id));
        const revisions = (connection: DatabaseConnection) =>
            connection
                .select({ noteId: revision.noteId, number: revision.number, title: revision.title })
                .from(revision)
                .orderBy(asc(revision.noteId), asc(revision.number));

        // write two notes and a revision, and remember how they were
        const editedAt = new Date(1_790_000_000_000);
        const views = 9_007_199_254_740_993n;
        await database.insert(note).values([
            { id: "a", title: "Plans", scope: "f", summary: null, views, labels: ["x"], editedAt },
            { id: "b", title: "Ideas", scope: "f", summary: "s", views: 1n, labels: [], editedAt },
        ]);
        await database.insert(revision).values({ scope: "f", noteId: "a", number: 1, title: "P" });
        const position = await database.log.position();
        const then = [await notes(database), await revisions(database)];

        // change one note twice, delete the other, add a third, and revise again
        await database.update(note).set({ title: "Travel", summary: "t" }).where(eq(note.id, "a"));
        await database
            .update(note)
            .set({ labels: ["y", "z"] })
            .where(eq(note.id, "a"));
        await database.delete(note).where(eq(note.id, "b"));
        await database.insert(note).values({
            id: "c",
            title: "New",
            scope: "f",
            summary: null,
            views: 2n,
            labels: [],
            editedAt,
        });
        await database.update(revision).set({ title: "Q" }).where(eq(revision.number, 1));

        // read the past exactly as it was, and the present at the latest position
        const past = await database.at(position);
        expect([await notes(past), await revisions(past)]).toEqual(then);
        const present = await database.at(await database.log.position());
        expect([await notes(present), await revisions(present)]).toEqual([
            await notes(database),
            await revisions(database),
        ]);

        // filter the past like the present
        expect(
            await past.select({ title: note.title }).from(note).where(eq(note.summary, "s")),
        ).toEqual([{ title: "Ideas" }]);

        // bound a transaction's changes by the positions around it
        const before = await database.log.latest();
        await database.transaction(async (transaction) => {
            await transaction.update(note).set({ summary: "u" }).where(eq(note.id, "a"));
            await transaction.update(note).set({ summary: "v" }).where(eq(note.id, "c"));
        });
        const bounds = await database.log.bounds(before + 1);
        expect([bounds.before, bounds.after, bounds.startedAt <= bounds.committedAt]).toEqual([
            before,
            before + 2,
            true,
        ]);

        // refuse writing the past and reading a position of another epoch
        await expect(past.delete(note).where(eq(note.id, "a"))).rejects.toThrow(
            new DatabaseError("READ_ONLY", "a database as of a position only reads"),
        );
        await expect(database.at({ ...position, epoch: "elsewhere" })).rejects.toMatchObject({
            code: "STALE_EPOCH",
        });
    },
);
