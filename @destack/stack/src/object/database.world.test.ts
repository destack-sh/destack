import { eq, Snapshot } from "@destack/db";
import { nth, Digest, Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Package, PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ids, insertRelease, insertRollout, readStack, TestSpace } from "../test/index.ts";
import { STACKS } from "@destack/account/test";
import { Database, database, installationRevision, rollout } from "./index.ts";
import { installation } from "@destack/object";
import objectDefinition from "@destack/object/destack.json" with { type: "json" };
import { describeObject } from "@destack/object/inspect";
import { note } from "../test/schema.ts";
import { RESOURCE_KIND } from "@destack/resource";

/** The tables the notes installation asks main for. */
const STATE = { tables: { sqlite: [], postgresql: [] } };

test("serve an installation once its targets are bound and the space's main database applied its tables, while a later plan waits, a stack at once, and none without main", async () => {
    // install notes at a revision declaring a note type, bound to main
    const fixture = await TestSpace.open();
    const { database: connection } = fixture;
    const now = fixture.machine.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const main = nth(await connection.select().from(database.table), 0);
    const stack = await readStack(connection);
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
    const state = await fixture.blobs.writeJson(STATE);
    await connection.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                ...record,
                id: revision,
                installationId: ids.notes,
                buildId: pulled.id,
                dependencies: [],
                states: { main: state },
                generation: 1,
                bindings: [],
                digest: Digest.parse("e".repeat(64)),
            }),
    );
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ revisionId: revision })
                .where(eq(rollout.table.installationId, ids.notes)),
    );
    const notes = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [
            {
                kind: "object",
                kindPackageId: objectDefinition.id,
                name: "note",
                description: describeObject(note),
            },
        ],
        fixture.environment.crypto,
    );
    const builds = await fixture.builds();
    const open = async (packageId: PackageId, manifest: Digest) =>
        manifest === release.manifest ? notes : builds.pull(fixture.registry, packageId, manifest);
    const served = async (
        opening: (
            packageId: PackageId,
            manifest: Digest,
        ) => Promise<Pick<BuildReader, "declarations">> = open,
    ) =>
        (await Database.installed(Snapshot.live(connection), ids.space, opening))
            .map((entry) => [
                entry.rollout.installationId,
                entry.declarations.map((each) => each.name),
            ])
            .toSorted(([left], [right]) => String(left).localeCompare(String(right)));

    // open the stack's build with a vault it declares for its document, which nothing binds to it
    const vaulted = async (packageId: PackageId, manifest: Digest) => {
        const built = await open(packageId, manifest);
        if (packageId !== stack.installation.packageId) {
            return built;
        }
        const declarations = await built.declarations();
        const vault = {
            ...RESOURCE_KIND.identity,
            name: "credentials",
            description: {},
        };

        return {
            declarations: async () => [...declarations, { ...nth(declarations, 0), ...vault }],
        };
    };

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
                .update(installation.table)
                .set({ conditions: { Bound: isBound } })
                .where(eq(installation.table.id, ids.notes)),
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
    const withVault = await served(vaulted);

    // serve nothing in a space without main
    await connection.transaction(
        async (transaction) =>
            await transaction.delete(database.table).where(eq(database.table.id, main.id)),
    );
    const unborn = await served();

    // serve the space's stack throughout, its document's vault unbound, and notes once bound and applied
    const stacked = [stack.installationId, [STACKS.space.export]];
    expect({ unbound, unapplied, applied, waiting, withVault, unborn }).toEqual({
        unbound: [stacked],
        unapplied: [stacked],
        applied: [[ids.notes, ["note"]], stacked],
        waiting: [[ids.notes, ["note"]], stacked],
        withVault: [
            [ids.notes, ["note"]],
            [stack.installationId, [STACKS.space.export, "credentials"]],
        ],
        unborn: [],
    });
});
