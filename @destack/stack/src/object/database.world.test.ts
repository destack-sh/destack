import { eq, Snapshot } from "@destack/db";
import { nth, Digest, Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { ids, insertRelease, insertRollout, TestSpace } from "../test/index.ts";
import { STACKS } from "@destack/account/test";
import { binding, Database, database, installationRevision, rollout } from "./index.ts";

/** The tables the notes installation asks main for. */
const STATE = { tables: { sqlite: [], postgresql: [] } };

test("serve an installation once its targets are bound and the space's main database applied its tables, while a later plan waits too, and none before or without main", async () => {
    // install notes at a revision declaring a note type, bound to main
    const fixture = await TestSpace.open();
    const { database: connection } = fixture;
    const now = fixture.machine.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const main = nth(await connection.select().from(database.table), 0);
    const stack = nth(
        await connection.select().from(rollout.table).where(eq(rollout.table.kind, "stack")),
        0,
    );
    await insertRollout(
        connection,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            installer: fixture.owner.subject,
        },
        record.createdAt,
    );
    const revision = Identifier.create("installation-revision", fixture.environment);
    const release = {
        package: ids.package,
        version: "2026.9.0",
        manifest: Digest.parse("d".repeat(64)),
    };
    const pulled = await insertRelease(
        connection,
        ids.space,
        release,
        [],
        fixture.machine.environment,
    );
    await connection.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                ...record,
                id: revision,
                installationId: ids.notes,
                buildId: pulled.id,
                dependencies: [],
                digest: Digest.parse("e".repeat(64)),
            }),
    );
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ revisionId: revision })
                .where(eq(rollout.table.id, ids.notes)),
    );
    await connection.transaction(
        async (transaction) =>
            await transaction.insert(binding.table).values({
                ...record,
                id: Identifier.create("binding", fixture.environment),
                installationId: ids.notes,
                packageId: ids.package,
                name: "main",
                target: main.id,
                state: STATE,
            }),
    );
    const notes = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [{ kind: "object", kindPackageId: ids.package, name: "note", description: {} }],
        fixture.environment.crypto,
    );
    const builds = await fixture.builds();
    const open = async (manifest: Digest) =>
        manifest === release.manifest ? notes : builds.packages.contents(manifest);
    const served = async () =>
        (
            await Database.installed(
                Snapshot.live(connection),
                ids.space,
                open,
                connection.state.environment.crypto,
            )
        )
            .map((entry) => [entry.rollout.id, entry.declarations.map((each) => each.name)])
            .toSorted(([left], [right]) => String(left).localeCompare(String(right)));

    // serve nothing while the notes' targets are not bound, though main applied their tables
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({
                    states: [await Digest.json(STATE, connection.state.environment.crypto)],
                    appliedStates: [await Digest.json(STATE, connection.state.environment.crypto)],
                })
                .where(eq(database.table.id, main.id)),
    );
    const unbound = await served();

    // serve nothing while main has not applied the notes' tables, their targets bound
    const isBound = {
        status: "true" as const,
        reason: "Bound",
        message: "",
        observedGeneration: 1,
        lastTransitionAt: now,
    };
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ conditions: { Bound: isBound } })
                .where(eq(rollout.table.id, ids.notes)),
    );
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({ appliedStates: [] })
                .where(eq(database.table.id, main.id)),
    );
    const unapplied = await served();

    // serve notes once main applied them
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({
                    appliedStates: [await Digest.json(STATE, connection.state.environment.crypto)],
                })
                .where(eq(database.table.id, main.id)),
    );
    const applied = await served();

    // serve notes while a plan for another state waits for its approval
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({
                    states: [
                        await Digest.json(STATE, connection.state.environment.crypto),
                        Digest.parse("f".repeat(64)),
                    ],
                })
                .where(eq(database.table.id, main.id)),
    );
    const waiting = await served();

    // serve nothing in a space without main
    await connection.transaction(async (transaction) => await transaction.delete(binding.table));
    await connection.transaction(
        async (transaction) =>
            await transaction.delete(database.table).where(eq(database.table.id, main.id)),
    );
    const unborn = await served();

    // serve the space's stack throughout, and notes once its targets are bound and main applied it
    const stacked = [stack.id, [STACKS.space.export]];
    expect({ unbound, unapplied, applied, waiting, unborn }).toEqual({
        unbound: [stacked],
        unapplied: [stacked],
        applied: [[ids.notes, ["note"]], stacked],
        waiting: [[ids.notes, ["note"]], stacked],
        unborn: [],
    });
});
