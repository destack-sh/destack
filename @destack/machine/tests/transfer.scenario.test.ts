import { UNIVERSE_NODE } from "@destack/universe/test";
import { RetryPolicy } from "@destack/environment";
import { MAIN_DATABASE } from "@destack/stack/declare";
import { JWS_ALGORITHMS, Jwks, Jwt, Recipient } from "@destack/identity";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, refusal, test } from "@destack/test";
import { Identity, Placement, placement, machine } from "@destack/account/object";
import {
    and,
    defineTable,
    eq,
    integer,
    TABLE,
    text,
    DatabaseKind,
    defineDatabase,
    DatabaseState,
} from "@destack/db";
import { type DatabaseConnection, databaseProvider } from "@destack/db/database";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { ResourceId, ResourceRecord } from "@destack/resource";
import { nth, Digest, Identifier, schema, present } from "@destack/schema";
import { RequestId, Scope, ObjectReference } from "@destack/sync";
import { ProviderIndex, SpaceTransfer } from "../src/index.ts";
import {
    fixtureBuild,
    ids,
    insertInstallation,
    TestRuntime,
    runNotes,
    TestSpace,
} from "../src/test/index.ts";
import * as object from "@destack/stack/object";
import * as accountObject from "@destack/account/object";
import { principal } from "@destack/access";
import { createClient } from "@destack/object/client";
import stackPackage from "@destack/stack/package";
import type { StackClient } from "@destack/stack/client";

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60 * 1000;

/** How long an unreferenced blob stays before deletion: a blob store's hour of grace. */
const GRACE_MILLISECONDS = 60 * 60_000;

/** A signal that never aborts, for waits the world bounds. */
const FOREVER = new AbortController().signal;

/** A machine the universe knows nothing of. */
const UNKNOWN_MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-00000000000b");

/** The origin the target machine answers at. */
const TARGET_ORIGIN = "http://target.test/";

/** The longest a machine may take to destroy a cancelled move's copy, in milliseconds. */
const ABANDON_MILLISECONDS = 600_000;

/** The notes an installation stores in the space's database resource. */
const note = defineTable(
    "note",
    {
        /** The note's key. */
        id: text("id").primaryKey(),
        /** The space the note lives in. */
        scope: text("scope").notNull(),
        /** The note's text. */
        body: text("body").notNull(),
        /** Its rank. */
        rank: integer("rank").notNull(),
    },
    { log: {} },
);

/** The pins an installed package stores in the space's main database, with the space's tables. */
const pin = defineTable(
    "pin",
    {
        /** The space of the pin. */
        scope: text("scope").primaryKey(),
        /** The pin's identifier. */
        id: text("id").primaryKey(),
        /** The pin's label. */
        label: text("label").notNull(),
    },
    { log: {} },
);

/** The database resource that stores the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

test("move a space and back, each source destroying its copy", async () => {
    // place the space on the fixture's cloud machine and an empty database on the target host
    const { source: sourceMachine, target, targetId, universe } = await openMove();
    const built = await sourceMachine.recordBuild(
        await fixtureBuild(ids.package, "2026.9.0", sourceMachine.environment.crypto),
    );
    // provision the notes database through the space's stack
    const world = sourceMachine.environment;
    const sourceDatabases = join(sourceMachine.files, "databases");
    const targetDatabases = join(target.files, "databases");
    const sourceHost = new BunSqliteDatabaseHost(pathToFileURL(`${sourceDatabases}/`), world);
    await sourceMachine.serve();
    await sourceMachine.apply([], { databases: { notes: notes.provision() } });
    const [declared] = await sourceMachine.database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.name, "notes"));
    const resource = present(declared, "the notes database");

    // add an unbound scratch database the source deletes before its fence
    const sources = databaseProvider(sourceHost, object.database);
    const now = sourceMachine.machine.clock.now();
    const scratchId = schema
        .identifier("database")
        .parse("database-01996ab0-0000-7000-8000-00000000000e");
    const scratchDeclared = {
        id: scratchId,
        scope: ids.space,
        name: "scratch",
        definitionPackageId: ids.package,
        definitionVersion: "2026.9.0",
        definitionName: "notes",
        spec: { replicated: [] },
        createdAt: now,
        updatedAt: now,
    };
    const scratchProvision = await sources.provision.provision(
        ResourceRecord.of(DatabaseKind, { ...scratchDeclared, reference: null }),
    );
    const scratch = {
        ...scratchDeclared,
        ...ObjectReference.columns(
            "owner",
            principal.space.reference(Scope.universe.id, ids.space),
        ),
        reference: scratchProvision.reference,
        provider: "sqlite",
        appliedStates: [],
    };
    await sourceMachine.database.transaction(
        async (transaction) => await transaction.insert(object.database.table).values(scratch),
    );

    // let the space's owner create a transfer to the target
    await sourceMachine
        .as(sourceMachine.owner.subject)
        .stepUp()
        .client(stackPackage)
        .transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer", sourceMachine.machine.environment),
            requestId: RequestId.create(sourceMachine.machine.environment),
            targetId: targetId,
        });

    // receive it on the target, deleting the scratch database on the source as the target asks it to fence
    const fencing = target.simulation.network.pause(targetId, sourceMachine.machine.id, (sent) =>
        sent.endsWith("/fence"),
    );
    const deleting = fencing.waiting.then(async () => {
        await sourceMachine.database.transaction((transaction) =>
            transaction
                .delete(object.database.table)
                .where(eq(object.database.table.id, scratchId))
                .execute(),
        );
        fencing.resume();
    });
    await targeted(target, targetId);
    await target.serve();
    const refused = await runMove(
        sourceMachine,
        target,
        (placed) => placed?.machine === targetId,
        1,
    );
    await deleting;
    expect(refused).toEqual([]);

    // let the source machine destroy the space it left: its volume, database files and blobs
    await sourceMachine.simulation.idle();
    const removed = {
        isVolumeKept: (await sourceMachine.machine.volumes.list()).includes(ids.space),
        databases: await world.files.exists(join(sourceDatabases, ids.space)),
        blobs: await world.files.exists(join(sourceMachine.files, "blobs", ids.space)),
    };

    // read the space, its notes and its records on the target
    const kept = await target.database
        .select()
        .from(object.database.table)
        .orderBy(object.database.table.name);
    const replicated = present(
        kept.find((entry) => entry.name === "notes"),
        "the replicated database",
    );
    const record = nth(await target.database.select().from(object.transfer.table), 0);
    const [recorded] = await target.database
        .select({ image: object.build.table.image })
        .from(object.build.table)
        .where(eq(object.build.table.id, built.build));
    const image = present(recorded?.image, "the moved build's image");
    const spaces = await target.database
        .select({ id: accountObject.space.table.id })
        .from(accountObject.space.table)
        .where(and(eq(accountObject.space.table.id, ids.space), accountObject.Space.served()));
    const targetKeystore = (await target.space).keystore;
    const signed = await targetKeystore.sign(target.database, ids.space, {
        sub: ids.space,
    });
    expect({
        removed,
        placement: await placementOf(universe),
        completed: [record.completedAt !== null, record.sourceId, record.targetId],
        moved: await (await target.builds()).contains(image),
        reference: replicated.reference,
        kept: kept.map((entry) => [entry.name, entry.provider]),
        databases: (await world.files.readDirectory(join(targetDatabases, ids.space)))
            .map((entry) => entry.name)
            .filter((name) => name.endsWith(".db")),
        spaces,
        signer: (
            await Jwt.verify(
                signed,
                Jwks.local(
                    await Identity.keys(universe.snapshot(), ids.space, world.crypto),
                    world.crypto,
                ),
                { algorithms: JWS_ALGORITHMS, now: sourceMachine.machine.clock.now() },
                world.crypto,
            )
        ).claims.sub,
    }).toEqual({
        // remove the source's whole volume
        removed: { isVolumeKept: false, databases: false, blobs: false },
        placement: { space: ids.space, scope: ids.account, machine: targetId, epoch: 2 },
        completed: [true, sourceMachine.machine.id, targetId],
        // replicate the space's build into the target's build store
        moved: true,
        reference: `${ids.space}/${resource.id}.db`,
        // store the main and notes databases in the target's storage
        kept: [
            ["main", "space"],
            ["notes", "sqlite"],
        ],
        databases: [`${resource.id}.db`],
        spaces: [{ id: ids.space }],
        signer: ids.space,
    });

    // move the space back to its source and to the target again, each having destroyed its copy once the space left it
    const sourceId = sourceMachine.machine.id;
    await moveSpace(target, sourceMachine, sourceId);
    await moveSpace(sourceMachine, target, targetId);
    const transfers = await target.database
        .select({
            sourceId: object.transfer.table.sourceId,
            sourceEpoch: object.transfer.table.sourceEpoch,
            outcome: object.transfer.table.outcome,
        })
        .from(object.transfer.table)
        .orderBy(object.transfer.table.sourceEpoch);
    expect({ placement: await placementOf(universe), transfers }).toEqual({
        placement: { space: ids.space, scope: ids.account, machine: targetId, epoch: 4 },
        transfers: [
            { sourceId, sourceEpoch: 1, outcome: "succeeded" },
            { sourceId: targetId, sourceEpoch: 2, outcome: "succeeded" },
            { sourceId, sourceEpoch: 3, outcome: "succeeded" },
        ],
    });
});

test("release a moved space's sockets on its source, so a closing client reaches nothing it left", async () => {
    // connect the source machine to the space's stack over a socket it accepts
    const { source, target, targetId } = await openMove();
    await source.serve();
    const connection = await source.machine.connection(stackPackage, ids.space);
    await connection.list(object.installation, {});

    // move the space to the target, then close the source's socket to it
    await moveSpace(source, target, targetId);
    connection.close();
    await source.simulation.idle();

    // refuse nothing at the close, as the move released the socket on the source
    expect(source.takeRefusals()).toEqual([]);
});

test("refuse a transfer without step-up or to a foreign machine", async () => {
    // replicate one machine of another account next to the fixture's cloud machine
    const fixture = await TestSpace.open();
    const foreign = schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-00000000000c");
    const other = schema
        .identifier("account")
        .parse("account-01996ab0-0000-7000-8000-00000000000d");
    const now = fixture.machine.clock.now();
    await fixture.database.transaction(
        async (transaction) =>
            await transaction.insert(machine.table).values({
                id: foreign,
                scope: other,
                name: "foreign",
                kind: "self",
                platform: "macOS",
                architecture: "arm64",
                model: "Mac14,2",
                createdAt: now,
                updatedAt: now,
            }),
    );
    await fixture.serve();

    // refuse each transfer with its reason
    const owner = fixture.as(fixture.owner.subject);
    const refused = (target: string, isAuthenticatedNow: boolean) =>
        refusal(
            (isAuthenticatedNow ? owner.stepUp() : owner).client(stackPackage).transfer.create({
                spaceId: ids.space,
                id: Identifier.create("transfer", fixture.environment),
                requestId: RequestId.create(fixture.machine.environment),
                targetId: target,
            }),
        );
    expect([
        await refused(UNKNOWN_MACHINE, false),
        await refused(UNKNOWN_MACHINE, true),
        await refused(foreign, true),
    ]).toEqual([
        [
            "UNAUTHORIZED",
            "ASSURANCE_UNAUTHENTICATED",
            "authentication required: authentication assurance is too low",
        ],
        [
            "BAD_REQUEST",
            "MACHINE_INVALID",
            `machine '${UNKNOWN_MACHINE}' is invalid: it is not a standing machine of the space's account or one hosting it`,
        ],
        [
            "BAD_REQUEST",
            "MACHINE_INVALID",
            `machine '${foreign}' is invalid: it is not a standing machine of the space's account or one hosting it`,
        ],
    ]);
});

test("move every table with a space's rows and the tables main applied, leaving the universe's and the machine's", async () => {
    // plan the move from the fixture's store, before main applies any package's tables
    await using fixture = await TestSpace.open();
    const { database } = fixture;
    const moving = { space: ids.space, scope: ids.account, machine: fixture.machine.id, epoch: 1 };
    const steps = await SpaceTransfer.of(await fixture.store(), moving, UNKNOWN_MACHINE);

    // move the transfer table first and each table with a scope column once,
    //  leaving out the tables the universe replicates here and the machine's own
    const first = steps.space.tables.map((table) => table[TABLE].sqlName);
    const scoped = database.tables
        .filter((table) => table[TABLE].columns["scope"] !== undefined)
        .map((table) => table[TABLE].sqlName);
    expect({
        first: first[0],
        isMovedOnce: new Set(first).size === first.length,
        unscoped: first.filter((name) => !scoped.includes(name)),
        left: scoped.filter((name) => !first.includes(name)).toSorted(),
    }).toEqual({
        first: "destack__stack__transfer",
        isMovedOnce: true,
        unscoped: [],
        left: [
            "destack__access__grant",
            "destack__account__account",
            "destack__account__assignment",
            "destack__account__client",
            "destack__account__hostname",
            "destack__account__key",
            "destack__account__machine",
            "destack__account__organization",
            "destack__account__placement",
            "destack__account__space",
            "destack__account__user",
            "destack__connection__connection",
            "destack__connection__integration",
            "destack__finance__product",
            "destack__finance__product_feature",
            "destack__finance__quota",
            "destack__finance__seller",
            "destack__finance__subscription",
            "destack__finance__subscription_item",
            "destack__object__claim",
            "destack__object__follow_key",
            "destack__object__journal",
            "destack__object__replica_request",
            "destack__object__settlement",
            "destack__registry__reference",
            "destack__registry__tag",
            "destack__stack__restore",
            "destack__stack__snapshot",
            "destack__stack__usage",
            "destack__sync__replica",
            "destack__sync__replica_key",
            "destack__sync__replica_page",
            "destack__sync__replica_position",
            "destack__sync__replica_result",
            "destack__sync__replica_scope",
            "destack__sync__replica_subscription",
            "destack__sync__subscription",
        ],
    });

    // apply an installed package's pins to the space's provisioned main database
    const pinState = DatabaseState.of([pin]);
    const digest = await Digest.json(pinState, fixture.environment.crypto);
    await (await fixture.space).blobs.writeJson(pinState);
    const main = and(
        eq(object.database.table.scope, ids.space),
        eq(object.database.table.name, MAIN_DATABASE),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({
                    states: [digest],
                    appliedStates: [digest],
                    provider: "space",
                    reference: "space",
                })
                .where(main),
    );

    // plan the move with the pins and the space's tables, and read their state at the handoff
    const states = (await SpaceTransfer.main(await fixture.space, ids.space))?.states;
    const moved = names(await SpaceTransfer.of(await fixture.space, moving, UNKNOWN_MACHINE));
    const own = names(await SpaceTransfer.of(await fixture.space, moving, UNKNOWN_MACHINE, []));

    // read only the applied states while the main database waits to apply its desired ones
    await database.transaction(
        async (transaction) =>
            await transaction.update(object.database.table).set({ appliedStates: [] }).where(main),
    );
    const waiting = (await SpaceTransfer.main(await fixture.space, ids.space))?.states;
    const pending = names(await SpaceTransfer.of(await fixture.space, moving, UNKNOWN_MACHINE));

    // move the space's own tables and then the pins, a plan waiting or not
    expect({ states, moved, waiting, pending }).toEqual({
        states: [pinState],
        moved: [...own, "destack__machine__pin"],
        waiting: [],
        pending: own,
    });
});

test("stop a moving space's instances once its fence starts", async () => {
    // run an instance of the notes installation in the space
    await using space = await TestSpace.open();
    const release = await space.release(ids.package, "2026.9.0");
    const running = await runNotes(
        space.database,
        release,
        await space.releaseRow(ids.package, "2026.9.0"),
        space.owner.subject,
        space.machine.environment,
    );
    const runtime = new TestRuntime({
        scheduler: space.machine.environment.scheduler,
        earlier: [running],
        builds: await space.builds(),
    });

    // fence the moving space on the machine running it
    const moving = { space: ids.space, scope: ids.account, machine: space.machine.id, epoch: 1 };
    const transfer = await SpaceTransfer.of(await space.space, moving, UNKNOWN_MACHINE);
    await transfer.fence(
        { providers: new ProviderIndex(space.machine.id, []), runtimes: [runtime] },
        space.machine.clock.now(),
    );
    expect(runtime.stopped).toEqual([running]);
});

test("cancel a fenced transfer and serve the space on its source", async () => {
    // let the space's owner move it to the target, whose machine is stopped so the test acts for it
    const { source: fixture, target, targetId, universe } = await openMove();
    const { database } = fixture;
    await target.stop();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).stepUp().client(stackPackage);
    const created = await owner.transfer.create({
        spaceId: ids.space,
        id: Identifier.create("transfer", fixture.environment),
        requestId: RequestId.create(fixture.machine.environment),
        targetId: targetId,
    });

    // let the target have the source fence once the source's replica shows the move, and stop as if its captures failed
    await database.log.waitUntil(
        async () => (await placementRow(database))?.targetId === targetId,
        FOREVER,
    );
    const moving = { spaceId: ids.space, id: created.id };
    await fixture
        .as(targetId)
        .client(stackPackage)
        .transfer.fence({
            ...moving,
            requestId: RequestId.create(fixture.machine.environment),
        });

    // let the source follow the target's receipt, then conceal the transfer from its cancellation
    await fixture.simulation.idle();
    const cancel = (caller: StackClient) =>
        caller.transfer.cancel({
            ...moving,
            requestId: RequestId.create(fixture.machine.environment),
        });
    const refused = await refusal(cancel(fixture.as(targetId).client(stackPackage)));

    // cancel as the owner
    await cancel(owner);

    // serve the space placed at the next epoch with no move
    const [placedAgain] = await universe
        .select({
            machine: placement.table.machineId,
            epoch: placement.table.epoch,
            target: placement.table.targetId,
        })
        .from(placement.table)
        .where(eq(placement.table.spaceId, ids.space));
    const [ended] = await database
        .select({
            completedAt: object.transfer.table.completedAt,
            outcome: object.transfer.table.outcome,
        })
        .from(object.transfer.table);
    const spaces = await database
        .select({ id: accountObject.space.table.id })
        .from(accountObject.space.table)
        .where(and(eq(accountObject.space.table.id, ids.space), accountObject.Space.served()));
    expect({
        refused,
        placement: placedAgain,
        outcome: [ended?.completedAt !== null, ended?.outcome],
        spaces,
        again: await refusal(cancel(owner)),
    }).toEqual({
        refused: ["NOT_FOUND", "NOT_FOUND", `transfer '${created.id}' not found`],
        placement: { machine: fixture.machine.id, epoch: 2, target: null },
        outcome: [true, "cancelled"],
        spaces: [{ id: ids.space }],
        again: [
            "PRECONDITION_FAILED",
            "TRANSFER_BLOCKED",
            `cannot cancel transfer '${created.id}': it has ended`,
        ],
    });
});

test("move a space afresh after a cancelled takeover", async () => {
    // add an enabled installation on the space's machine and an empty target
    const { source, target, targetId, universe } = await openMove();
    const now = source.machine.clock.now();
    await insertInstallation(
        source.database,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            version: await source.releaseRow(ids.package, "2026.9.0"),
            installer: source.owner.subject,
        },
        now,
    );

    // serve the space on the source as its owner
    await source.serve();
    const owner = createClient(
        stackPackage,
        source.as(source.owner.subject).stepUp().endpoint(stackPackage),
        source.machine.environment,
    );

    // move the space to the target, which the owner cancels once the target took it over and before it places it
    const created = await owner.transfer.create({
        spaceId: ids.space,
        id: Identifier.create("transfer", source.machine.environment),
        requestId: RequestId.create(source.machine.environment),
        targetId,
    });
    const paused = target.simulation.network.pause(targetId, UNIVERSE_NODE, (sent) =>
        sent.includes("/store/push"),
    );
    const cancelling = paused.waiting.then(async () => {
        // cancel as the owner before the target's first call to the universe, and let the target's machine see the placement no longer moves here
        await owner.transfer.cancel({
            spaceId: ids.space,
            id: created.id,
            requestId: RequestId.create(source.machine.environment),
        });
        const seen = target.machine.directory.snapshot.database;
        await seen.log.waitUntil(
            async () => (await placementRow(seen))?.targetId === null,
            FOREVER,
        );
        paused.resume();
    });
    await targeted(target, targetId);
    const cancelled = await runMove(
        source,
        target,
        (placed) => placed?.machine === source.machine.id && placed.epoch === 2,
    );
    await cancelling;

    // read the installation as the owner on a store of the target, refused on the abandoned target
    const read = (space: TestSpace) =>
        createClient(
            stackPackage,
            space.as(source.owner.id).endpoint(stackPackage),
            source.machine.environment,
        ).installation.get({ scope: ids.space, id: ids.notes });

    // let the target machine destroy the cancelled move's copy once the space no longer moves there
    await target.simulation.waitUntil(
        async () => (await target.machine.volumes.list()).length === 0,
        target.simulation.clock.now() + ABANDON_MILLISECONDS,
    );
    const abandoned = await target.machine.volumes.list();

    // move the space afresh to the same target as the owner, and read the installation as the target serves it placed there
    await owner.transfer.create({
        spaceId: ids.space,
        id: Identifier.create("transfer", source.machine.environment),
        requestId: RequestId.create(source.machine.environment),
        targetId,
    });
    await targeted(target, targetId);
    await target.serve();
    const moved = await runMove(source, target, (placed) => placed?.machine === targetId);
    const received = await target.database
        .select({
            sourceEpoch: object.transfer.table.sourceEpoch,
            outcome: object.transfer.table.outcome,
        })
        .from(object.transfer.table)
        .orderBy(object.transfer.table.sourceEpoch);
    const rollout = await read(await target.serve());
    expect({
        cancelled,
        abandoned,
        moved,
        placement: await placementOf(universe),
        transfers: received,
        installation: rollout.id,
    }).toEqual({
        cancelled: [],
        abandoned: [],
        moved: [],
        placement: { space: ids.space, scope: ids.account, machine: targetId, epoch: 3 },
        transfers: [
            { sourceEpoch: 1, outcome: "cancelled" },
            { sourceEpoch: 2, outcome: "succeeded" },
        ],
        installation: ids.notes,
    });
});

test("collect stray and expired snapshot blobs after a grace", async () => {
    // serve the fixture's space on the machine that stores the snapshots of it
    const fixture = await TestSpace.open({});
    const { database } = fixture;
    const { blobs } = await fixture.space;
    const start = fixture.machine.clock.now();

    // put a blob a taken snapshot references, and a stray blob none references
    const put = async (content: string) => {
        const bytes = new TextEncoder().encode(content);
        const digest = await Digest.of(bytes, fixture.environment.crypto);
        await blobs.put([
            {
                digest,
                size: bytes.byteLength,
                open: async () =>
                    (async function* () {
                        yield bytes;
                    })(),
            },
        ]);

        return digest;
    };
    const [taken] = [await put("taken"), await put("stray")];
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.snapshot.table).values({
                id: schema
                    .identifier("snapshot")
                    .parse(object.snapshot.generateId(fixture.environment)),
                spaceId: ids.space,
                resourceId: ResourceId.parse("database-01996ab0-0000-7000-8000-0000000000f1"),
                recipient: (await Recipient.generate(fixture.environment)).key,
                digest: taken,
                takenAt: start,
                blobs: [taken],
                createdAt: start,
                updatedAt: start,
            }),
    );

    // collect the stray blob a grace past its lease, then the snapshot's blob a grace after the snapshot expires, leaving the blobs main and the revisions refer to
    await fixture.simulation.jump(start + blobs.leaseMilliseconds + 1);
    await fixture.simulation.jump(fixture.simulation.clock.now() + GRACE_MILLISECONDS);
    const leased = await Array.fromAsync(blobs.storage.packs());
    await fixture.simulation.jump(start + 2 * DAY_MILLISECONDS);
    await fixture.simulation.jump(fixture.simulation.clock.now() + GRACE_MILLISECONDS);
    const [main] = await database
        .select({ states: object.database.table.states })
        .from(object.database.table)
        .where(eq(object.database.table.name, MAIN_DATABASE));
    const revisions = await database
        .select({ declarations: object.installationRevision.table.declarations })
        .from(object.installationRevision.table);
    const kept = [
        ...present(main, "the main database").states,
        ...revisions.map((revision) => revision.declarations),
    ];
    expect({
        leased: leased.toSorted(),
        expired: (await Array.fromAsync(blobs.storage.packs())).toSorted(),
        snapshots: await database.select().from(object.snapshot.table),
    }).toEqual({
        leased: [taken, ...kept].toSorted(),
        expired: [...kept].toSorted(),
        snapshots: [],
    });
});

/** Open the space on its region's machine and an empty target machine its owner enrolls. */
async function openMove() {
    // enroll the target as the owner, and host an empty database there next to the space's source
    const source = await TestSpace.open();
    const enrolled = await source.universe.enroll(source.owner, ids.account, TARGET_ORIGIN);
    const target = await TestSpace.empty({
        world: source,
        owner: source.owner,
        machine: enrolled,
    });

    // wait until the source machine's directory holds the target
    const isKnown = async () =>
        (await source.machine.directory.snapshot
            .read(accountObject.machine.table)
            .findFirst({ where: { id: enrolled.id } })) !== undefined;
    await source.simulation.waitUntil(isKnown);

    return {
        source,
        target,
        targetId: enrolled.id,
        universe: source.universe.storage.database,
    };
}

/** Move the space as its owner to another machine, which stands by until placed there. */
async function moveSpace(
    source: TestSpace,
    target: TestSpace,
    targetId: Identifier<"machine">,
): Promise<void> {
    // ask for the move as the space's owner on its source, once it follows the space's chain
    const isFollowed = async () =>
        (await Scope.read(source.database.snapshot(), [ids.account])).length > 0;
    await source.simulation.idle();
    await source.serve();
    await source.database.log.waitUntil(isFollowed, FOREVER);
    await source
        .as(source.owner.subject)
        .stepUp()
        .client(stackPackage)
        .transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer", source.machine.environment),
            requestId: RequestId.create(source.machine.environment),
            targetId,
        });

    // receive it on the target as a standby
    await targeted(target, targetId);
    await target.serve();
    const refused = await runMove(source, target, (placed) => placed?.machine === targetId);
    expect(refused).toEqual([]);
}

/** Run the world until a move's placement settles within the target's lag and expected retries. */
async function runMove(
    source: TestSpace,
    target: TestSpace,
    isSettled: (placed: Placement | undefined) => boolean,
    expected = 0,
): Promise<(readonly [string, string, string])[]> {
    // allow the target's access lag and the longest waits of the retries a test expects
    const universe = source.universe.storage.database;
    const { simulation } = source;
    const policy = RetryPolicy.of();
    const serving = (await placementOf(universe))?.machine === target.machine.id ? target : source;
    let budget = (await serving.store()).authorizer.lag;
    for (let failures = 1; failures <= expected; failures += 1) {
        budget += RetryPolicy.interval(policy, failures, () => 1);
    }

    // run until the placement settles, then take the target's refusals
    await simulation.waitUntil(
        async () => isSettled(await placementOf(universe)),
        simulation.clock.now() + budget,
    );
    const refused = target.takeRefusals();

    // let both machines' directories show the settled placement
    const settled = JSON.stringify(await placementOf(universe));
    for (const space of [source, target]) {
        const { snapshot } = space.machine.directory;
        const isShown = async () =>
            JSON.stringify((await Placement.find(snapshot, [ids.space]))[0]) === settled;
        await snapshot.database.log.waitUntil(isShown, FOREVER);
    }

    return refused;
}

/** Wait until a target's directory shows the space moving to it, as its machine starts it then. */
async function targeted(target: TestSpace, targetId: Identifier<"machine">): Promise<void> {
    const { snapshot } = target.machine.directory;
    await snapshot.database.log.waitUntil(
        async () => (await placementRow(snapshot.database))?.targetId === targetId,
        FOREVER,
    );
}

/** Read the space's placement row a database replicates, none while unplaced. */
async function placementRow(database: DatabaseConnection) {
    const [row] = await database
        .select()
        .from(placement.table)
        .where(eq(placement.table.spaceId, ids.space));

    return row;
}

/** Read the space's placement in the universe, none while unplaced. */
async function placementOf(universe: DatabaseConnection): Promise<Placement | undefined> {
    const [placed] = await Placement.find(universe.snapshot(), [ids.space]);

    return placed;
}

/** List the SQL names of the tables a transfer moves, in order. */
function names(transfer: SpaceTransfer): string[] {
    return transfer.space.tables.map((table) => table[TABLE].sqlName);
}
