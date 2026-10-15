import { eq, Snapshot } from "@destack/db";
import { aligned, Digest, Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { ids, StackFixture } from "../test/index.ts";
import {
    binding,
    Database,
    database,
    declaration,
    installation,
    installationRevision,
} from "./index.ts";

/** The tables the notes installation asks main for. */
const STATE = { tables: { sqlite: [], postgresql: [] } };

test("serve an installation once the space's main database applied its tables, and none before or without main", async () => {
    // install notes at a revision declaring a note type, bound to main
    const fixture = await StackFixture.open();
    const { database: connection } = fixture;
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const main = aligned(await connection.select().from(database.table), 0);
    await connection.insert(installation.table).values({
        ...record,
        id: ids.notes,
        packageId: ids.package,
        role: "application",
        selection: { kind: "release", version: "2026.9.0" },
        name: "notes",
    });
    const revision = Identifier.create("installation-revision");
    await connection.insert(installationRevision.table).values({
        ...record,
        id: revision,
        installationId: ids.notes,
        build: { kind: "release", version: "2026.9.0", manifest: "d".repeat(64) },
        views: {},
        settings: [],
        catalogs: [],
        digest: "e".repeat(64),
    });
    await connection
        .update(installation.table)
        .set({ revisionId: revision })
        .where(eq(installation.table.id, ids.notes));
    await connection.insert(declaration.table).values({
        ...record,
        id: Identifier.create("declaration"),
        revisionId: revision,
        installationId: ids.notes,
        packageId: ids.package,
        kindPackageId: ids.package,
        kind: "object",
        name: "note",
        description: {},
    });
    await connection.insert(binding.table).values({
        ...record,
        id: Identifier.create("binding"),
        installationId: ids.notes,
        packageId: ids.package,
        name: "main",
        target: main.id,
        state: STATE,
    });
    const served = async () =>
        (await Database.installed(Snapshot.live(connection), ids.space)).map((entry) => [
            entry.installation.id,
            entry.declarations.map((each) => each.name),
        ]);

    // serve nothing while main has not applied the notes' tables
    await connection
        .update(database.table)
        .set({ states: [await Digest.json(STATE)] })
        .where(eq(database.table.id, main.id));
    const unapplied = await served();

    // serve notes once main applied them
    await connection
        .update(database.table)
        .set({ appliedState: await Digest.json([await Digest.json(STATE)]) })
        .where(eq(database.table.id, main.id));
    const applied = await served();

    // serve nothing in a space without main
    await connection.delete(binding.table);
    await connection.delete(database.table).where(eq(database.table.id, main.id));
    const unborn = await served();

    expect({ unapplied, applied, unborn }).toEqual({
        unapplied: [],
        applied: [[ids.notes, ["note"]]],
        unborn: [],
    });
});
