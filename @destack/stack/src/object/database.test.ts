import { eq, Snapshot } from "@destack/db";
import { nth, Digest, Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { ids, insertRelease, StackFixture } from "../test/index.ts";
import { binding, Database, database, installation, installationRevision } from "./index.ts";

/** The tables the notes installation asks main for. */
const STATE = { tables: { sqlite: [], postgresql: [] } };

test("serve an installation once its targets are bound and the space's main database applied its tables, while a later plan waits too, and none before or without main", async () => {
    // install notes at a revision declaring a note type, bound to main
    const fixture = await StackFixture.open();
    const { database: connection } = fixture;
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const main = nth(await connection.select().from(database.table), 0);
    await connection.insert(installation.table).values({
        ...record,
        id: ids.notes,
        packageId: ids.package,
        role: "application",
        selection: { kind: "release", version: "2026.9.0" },
        name: "notes",
    });
    const revision = Identifier.create("installation-revision");
    const release = { package: ids.package, version: "2026.9.0", manifest: "d".repeat(64) };
    const pulled = await insertRelease(connection, ids.space, release);
    await connection.insert(installationRevision.table).values({
        ...record,
        id: revision,
        installationId: ids.notes,
        buildId: pulled.id,
        dependencies: [],
        digest: "e".repeat(64),
    });
    await connection
        .update(installation.table)
        .set({ revisionId: revision })
        .where(eq(installation.table.id, ids.notes));
    await connection.insert(binding.table).values({
        ...record,
        id: Identifier.create("binding"),
        installationId: ids.notes,
        packageId: ids.package,
        name: "main",
        target: main.id,
        state: STATE,
    });
    const notes = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [{ kind: "object", kindPackageId: ids.package, name: "note", description: {} }],
    );
    const open = async () => notes;
    const served = async () =>
        (await Database.installed(Snapshot.live(connection), ids.space, open)).map((entry) => [
            entry.installation.id,
            entry.declarations.map((each) => each.name),
        ]);

    // serve nothing while the notes' targets are not bound, though main applied their tables
    await connection
        .update(database.table)
        .set({ states: [await Digest.json(STATE)], appliedStates: [await Digest.json(STATE)] })
        .where(eq(database.table.id, main.id));
    const unbound = await served();

    // serve nothing while main has not applied the notes' tables, their targets bound
    const isBound = {
        status: "true" as const,
        reason: "Bound",
        message: "",
        observedGeneration: 1,
        lastTransitionAt: now,
    };
    await connection
        .update(installation.table)
        .set({ conditions: { bound: isBound } })
        .where(eq(installation.table.id, ids.notes));
    await connection
        .update(database.table)
        .set({ appliedStates: [] })
        .where(eq(database.table.id, main.id));
    const unapplied = await served();

    // serve notes once main applied them
    await connection
        .update(database.table)
        .set({ appliedStates: [await Digest.json(STATE)] })
        .where(eq(database.table.id, main.id));
    const applied = await served();

    // keep serving notes while a plan for another state waits for its approval
    await connection
        .update(database.table)
        .set({ states: [await Digest.json(STATE), "f".repeat(64)] })
        .where(eq(database.table.id, main.id));
    const waiting = await served();

    // serve nothing in a space without main
    await connection.delete(binding.table);
    await connection.delete(database.table).where(eq(database.table.id, main.id));
    const unborn = await served();

    expect({ unbound, unapplied, applied, waiting, unborn }).toEqual({
        unbound: [],
        unapplied: [],
        applied: [[ids.notes, ["note"]]],
        waiting: [[ids.notes, ["note"]]],
        unborn: [],
    });
});
