import { expect, test } from "@destack/test";
import { defineTable, text, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { Recipient } from "@destack/identity";
import { onTestFinished } from "@destack/test";
import { schema } from "@destack/schema";
import { Scope, replicaTables } from "@destack/sync";
import { SpaceReplica } from "./replica.ts";

/** The space that moves, with a room inside it. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000c1");

/** Notes stored in any scope. */
const note = defineTable(
    "move_note",
    {
        /** The note's key. */
        id: text("id").primaryKey(),
        /** The scope the note lives in. */
        scope: text("scope").notNull(),
    },
    { log: {} },
);

/** A scope row below a parent. */
function scopeRow(scope: string, parent: string) {
    return {
        scope,
        parent,
        partition: SPACE,
        ancestors: [parent],
        packageId: schema
            .identifier("package")
            .parse("package-01996ab0-0000-7000-8000-0000000000c2"),
        type: "folder",
    };
}

test("move the rows of a space and the scopes below it, digest them alike on both sides, and delete them all", async () => {
    // add a space, a room in it, a desk in the room and a scope outside, with a note in each
    const world = testSimulation().environment("test");
    const tables = [Scope.table, note, ...replicaTables];
    const source = await TestDatabase.create(tables, { environment: world, isMigrated: true });
    const target = await TestDatabase.create(tables, {
        environment: world,
        isMigrated: true,
        isReplica: true,
    });
    onTestFinished(async () => {
        await source.close();
        await target.close();
    });
    await source.database.transaction(
        async (transaction) =>
            await transaction
                .insert(Scope.table)
                .values([
                    scopeRow(SPACE, "account"),
                    scopeRow("room", SPACE),
                    scopeRow("desk", "room"),
                    scopeRow("other", "account"),
                ]),
    );
    await source.database.transaction(
        async (transaction) =>
            await transaction.insert(note).values(
                [SPACE, "room", "desk", "other"].map((scope) => ({
                    id: `note-${scope}`,
                    scope,
                })),
            ),
    );

    // add a stale note in a child scope on the target, as an earlier capture left it
    await target.database.transaction(
        async (transaction) =>
            await transaction.insert(note).values({ id: "note-stale", scope: "desk" }),
    );

    // capture the source's rows over the subtree it fenced and verify them against its digest
    const leaving = await SpaceReplica.of(source.database, SPACE, []);
    const scopes = await Scope.subtree(Snapshot.live(source.database), SPACE);
    const fenced = {
        position: await source.database.log.position(),
        digest: await SpaceReplica.digest(source.database, leaving.digestTables(), scopes),
        scopes,
    };
    const arriving = (await SpaceReplica.of(target.database, SPACE, [])).within(fenced.scopes);
    await arriving.capture(
        {
            capture: () =>
                leaving.feed().capture(leaving.replica.captured, AbortSignal.timeout(4000), {
                    seal: async (_table, row) => row,
                }),
        },
        fenced,
        await Recipient.generate(world),
        async (_table, row) => row,
    );
    const notes = async () =>
        (await target.database.select({ id: note.id }).from(note)).map((row) => row.id).toSorted();
    const moved = await notes();

    // move only the subtree's rows, then delete them on the target
    await arriving.delete();
    expect({ moved, deleted: await notes() }).toEqual({
        moved: ["note-desk", "note-room", `note-${SPACE}`].toSorted(),
        deleted: [],
    });
});
