import { RetryPolicy } from "@destack/environment";
import { MAIN_DATABASE } from "@destack/stack/declare";
import { PackageId } from "@destack/package";
import { JWS_ALGORITHMS, Jwk, Jwks, Jwt, PublicKey, Recipient } from "@destack/identity";
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
    type DatabaseConnection,
    Snapshot,
    DatabaseState,
    Lease,
    databaseProvider,
} from "@destack/db";
import { BunSqliteDatabaseHost, bunSqliteConnector } from "@destack/db/bun";
import { SqliteDatabase } from "@destack/db/sqlite";
import { desiredState } from "@destack/object";
import { Plan, ResourceId } from "@destack/resource";
import { nth, Digest, Identifier, schema, present } from "@destack/schema";
import { RequestId, Scope, type Uplink } from "@destack/sync";
import { Assertion } from "@destack/access/token";
import {
    ProviderIndex,
    serveSpace,
    sweepSnapshots,
    SWEEP_LEASE,
    SpaceTransfer,
    type Handoff,
} from "../src/server/index.ts";
import {
    fixtureBuild,
    ids,
    insertRollout,
    reconcile,
    TestRuntime,
    runNotes,
    TestSpace,
} from "../src/test/index.ts";
import { sharedNote } from "../src/test/schema.ts";
import objectPackage from "@destack/object/package";
import * as object from "../src/object/index.ts";
import * as accountObject from "@destack/account/object";
import { principal } from "@destack/access";
import { createClient } from "@destack/object/client";
import stackPackage from "../src/package.ts";
import type { StackClient } from "../src/client/index.ts";
import type { Environment } from "@destack/environment";
import { SimulatedClock } from "@destack/environment/simulation";
import type { ObjectStore } from "@destack/object/server";
import type { TestObjectStore } from "@destack/object/test";

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60 * 1000;

/** How long a test moving a space between two machines runs at most, beyond its longest wait. */
const SCENARIO_MILLISECONDS = 30_000;

/** The control loop's first retry of a failed reconciliation, as its default policy waits. */
const FIRST_RETRY_MILLISECONDS = RetryPolicy.of().initialInterval;

/** A signal that never aborts, for waits a settle bounds. */
const FOREVER = new AbortController().signal;

/** A machine the universe knows nothing of. */
const UNKNOWN_MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-00000000000b");

/** The origin the target machine answers at. */
const TARGET_ORIGIN = "http://target.test/";

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

/** Open a replica of the notes database where its host stores it, refusing another dialect. */
async function openNotes(
    id: ResourceId,
    reference: string,
    host: Pick<BunSqliteDatabaseHost, "resolve">,
    environment: Environment,
): Promise<SqliteDatabase> {
    const connection = await bunSqliteConnector.connect(
        { resource: id, kind: "database", provider: "sqlite", reference: host.resolve(reference) },
        notes,
        environment,
    );
    if (!(connection instanceof SqliteDatabase)) {
        throw new TypeError(`the notes database ${id} is no SQLite database`);
    }

    return connection;
}

test(
    "receive a moving space with its rows, databases and keys, then remove the source's replica",
    async () => {
        // place the space on the fixture's cloud machine and an empty database on the target host
        const { source: sourceMachine, target, targetId, universe } = await openMove();
        const source = sourceMachine.database;
        const built = await sourceMachine.recordBuild(
            await fixtureBuild(ids.package, "2026.9.0", sourceMachine.environment.crypto),
        );
        // store the space's keys under the source machine's keyring, which differs from the target's
        const { keystore: sourceKeystore } = await sourceMachine.options();
        const { keystore: targetKeystore } = await target.options();

        // apply a notes database with two notes on the source for an installation's binding
        const world = sourceMachine.environment;
        const files = await world.files.makeTemporaryDirectory("destack-space-transfer-");
        const sourceHost = new BunSqliteDatabaseHost(pathToFileURL(`${files}/source/`), world);
        const targetHost = new BunSqliteDatabaseHost(pathToFileURL(`${files}/target/`), world);
        const sources = databaseProvider(sourceHost, object.database);
        const targets = databaseProvider(targetHost, object.database);
        const now = sourceMachine.machine.clock.now();
        const declared = {
            id: schema
                .identifier("database")
                .parse("database-01996ab0-0000-7000-8000-00000000000c"),
            scope: ids.space,
            name: "notes",
            definitionPackageId: ids.package,
            definitionVersion: "2026.9.0",
            definitionName: "notes",
            spec: { replicated: [] },
            createdAt: now,
            updatedAt: now,
        };
        const state = notes.state();
        const desired = [state];
        const digest = await Digest.json(state, world.crypto);
        const provision = await sources.provision.provision(
            DatabaseKind.record({ ...declared, reference: null }),
        );
        const resource = {
            ...declared,
            reference: provision.reference,
            provider: "sqlite",
            states: [digest],
            appliedStates: [digest],
        };
        await source.transaction(
            async (transaction) =>
                await transaction.insert(desiredState).values({ scope: ids.space, digest, state }),
        );
        await source.transaction(
            async (transaction) => await transaction.insert(object.database.table).values(resource),
        );

        // add an unbound scratch database the source deletes before its fence
        const scratchId = schema
            .identifier("database")
            .parse("database-01996ab0-0000-7000-8000-00000000000e");
        const scratchDeclared = { ...declared, id: scratchId, name: "scratch" };
        const scratchProvision = await sources.provision.provision(
            DatabaseKind.record({ ...scratchDeclared, reference: null }),
        );
        const scratch = {
            ...scratchDeclared,
            reference: scratchProvision.reference,
            provider: "sqlite",
            appliedStates: [],
        };
        await source.transaction(
            async (transaction) => await transaction.insert(object.database.table).values(scratch),
        );
        const scratchRecord = DatabaseKind.record(scratch);
        await sources.reconcile.apply(
            scratchRecord,
            [],
            await Plan.digest(await sources.reconcile.plan(scratchRecord, []), world.crypto),
        );
        await insertRollout(
            source,
            {
                id: ids.notes,
                scope: ids.space,
                packageId: ids.package,
                name: "notes",
                selection: { kind: "release", version: "2026.9.0" },
                installer: sourceMachine.owner.subject,
            },
            now,
        );
        await source.transaction(
            async (transaction) =>
                await transaction.insert(object.binding.table).values({
                    id: schema
                        .identifier("binding")
                        .parse("binding-01996ab0-0000-7000-8000-00000000000d"),
                    scope: ids.space,
                    installationId: ids.notes,
                    packageId: ids.package,
                    name: "notes",
                    target: declared.id,
                    state,
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        const provisioned = DatabaseKind.record(resource);
        await sources.reconcile.apply(
            provisioned,
            desired,
            await Plan.digest(await sources.reconcile.plan(provisioned, desired), world.crypto),
        );
        const written = await openNotes(
            resource.id,
            resource.reference,
            sourceHost,
            sourceMachine.machine.environment,
        );
        await written.transaction(
            async (transaction) =>
                await transaction.insert(note).values([
                    { id: "n1", scope: ids.space, body: "first", rank: 1 },
                    { id: "n2", scope: ids.space, body: "second", rank: 2 },
                ]),
        );
        await written.close();

        // serve the space and let its owner create a transfer to the target
        const serving = await sourceMachine.serve({
            providers: [sources, ...(await sourceMachine.options()).providers],
            keystore: sourceKeystore,
        });
        const announcing = serving.store;
        const created = await serving
            .as(sourceMachine.owner.subject)
            .stepUp()
            .client(stackPackage)
            .transfer.create({
                spaceId: ids.space,
                id: Identifier.create("transfer", sourceMachine.machine.environment),
                requestId: RequestId.create(sourceMachine.machine.environment),
                targetId: targetId,
            });
        await reconcile(announcing, "transfer", { id: created.id });

        // receive it on the target and cut the end off the first capture
        let isCut = true;
        const handoff = async (from: string): Promise<Handoff> => {
            expect(from).toBe(sourceMachine.machine.id);
            const handed = target.handoff(serving, serving.as(targetId));

            return {
                ...handed,
                fence: async (moving) => {
                    // delete the scratch database on the source before the fence while the target follows it
                    await source.transaction((transaction) =>
                        transaction
                            .delete(object.database.table)
                            .where(eq(object.database.table.id, scratchId))
                            .execute(),
                    );

                    return handed.fence(moving);
                },
                capture: async function* (input, options) {
                    // cut the end off the first capture of the space's rows
                    const isCutting = isCut;
                    isCut = false;
                    for await (const page of handed.capture(input, options)) {
                        if (!isCutting || !page.complete) {
                            yield page;
                        }
                    }
                },
            };
        };
        // serve the space as the target's standby, following it as its placement moves here, until the universe places it there
        const receiving = await target.serve({
            role: "standby",
            handoff,
            providers: [targets, ...(await target.options()).providers],
        });
        const refused = await settleMove(
            sourceMachine,
            serving,
            target,
            receiving,
            (placed) => placed?.machine === targetId,
            1,
        );
        expect(refused).toEqual([]);

        // stop serving the space as its primary, then remove the source's replica as it stands by, and sweep its store a day later
        await serving[Symbol.asyncDispose]();
        const { store: standing } = await sourceMachine.serve({
            role: "standby",
            providers: [sources, ...(await sourceMachine.options()).providers],
            keystore: sourceKeystore,
        });
        await reconcile(standing, "transfer", { id: created.id });
        const sourceStore = sourceMachine.store();
        const blobs = sourceStore.size;
        await sweepSnapshots(
            at(announcing, sourceMachine.machine.clock.now() + 2 * DAY_MILLISECONDS),
            sourceStore,
        );
        const removed = {
            databases: await source.select().from(object.database.table),
            snapshots: await source.select().from(object.snapshot.table),
            files: (await world.files.readDirectory(join(files, "source", ids.space)))
                .map((entry) => entry.name)
                .filter((name) => name.endsWith(".db")),
            blobs: [blobs > 0, sourceStore.size],
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
        const read = await openNotes(
            replicated.id,
            present(replicated.reference, "the replica's reference"),
            targetHost,
            target.machine.environment,
        );
        const record = nth(await target.database.select().from(object.transfer.table), 0);
        const [recorded] = await target.database
            .select({ manifest: object.build.table.manifest })
            .from(object.build.table)
            .where(eq(object.build.table.id, built.build));
        const manifest = present(recorded?.manifest, "the moved build's manifest");
        const served = await target.database
            .select({ id: accountObject.space.table.id })
            .from(accountObject.space.table)
            .where(and(eq(accountObject.space.table.id, ids.space), accountObject.Space.served()));
        const signed = await targetKeystore.sign(target.database, ids.space, {
            sub: ids.space,
        });
        expect({
            removed,
            placement: await placementOf(universe),
            notes: await read.select().from(note).orderBy(note.id),
            completed: [record.completedAt !== null, record.sourceId, record.targetId],
            moved: await (await target.builds()).packages.contains(manifest),
            reference: replicated.reference,
            kept: kept.map((entry) => [entry.name, entry.provider]),
            databases: (await world.files.readDirectory(join(files, "target", ids.space)))
                .map((entry) => entry.name)
                .filter((name) => name.endsWith(".db")),
            served,
            signer: (
                await Jwt.verify(
                    signed,
                    Jwks.local(
                        await Identity.keys(Snapshot.live(universe), ids.space, world.crypto),
                        world.crypto,
                    ),
                    { algorithms: JWS_ALGORITHMS, now: sourceMachine.machine.clock.now() },
                    world.crypto,
                )
            ).claims.sub,
        }).toEqual({
            // remove the scratch database's file alone
            removed: {
                databases: [],
                snapshots: [],
                files: [`${scratchId}.db`],
                blobs: [true, 0],
            },
            placement: { id: ids.space, scope: ids.account, machine: targetId, epoch: 2 },
            notes: [
                { id: "n1", scope: ids.space, body: "first", rank: 1 },
                { id: "n2", scope: ids.space, body: "second", rank: 2 },
            ],
            completed: [true, sourceMachine.machine.id, targetId],
            // replicate the space's build into the target's build store
            moved: true,
            reference: `${ids.space}/${declared.id}.db`,
            // store the main and notes databases in the target's storage
            kept: [
                ["main", "space"],
                ["notes", "sqlite"],
            ],
            databases: [`${declared.id}.db`],
            served: [{ id: ids.space }],
            signer: ids.space,
        });
        await read.close();
    },
    SCENARIO_MILLISECONDS,
);

test("refuse a transfer without a recent sign-in, to an unknown machine, or to a machine of another account", async () => {
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
                kind: "personal",
                platform: "macOS",
                architecture: "arm64",
                model: "Mac14,2",
                createdAt: now,
                updatedAt: now,
            }),
    );
    const serving = await fixture.serve();

    // refuse each transfer with its reason
    const owner = serving.as(fixture.owner.subject);
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
            "AUTHENTICATION_INSUFFICIENT",
            "authentication assurance is too low: authenticate again",
        ],
        [
            "BAD_REQUEST",
            "MACHINE_UNKNOWN",
            `${UNKNOWN_MACHINE} is not a standing machine of the space's account or residency`,
        ],
        [
            "BAD_REQUEST",
            "MACHINE_UNKNOWN",
            `${foreign} is not a standing machine of the space's account or residency`,
        ],
    ]);
});

test("move every table of the space database with rows in spaces, leaving out the machine's own, replicated and unlogged tables", async () => {
    // declare a space database with another package's space-scoped notes next to the space's tables
    const fixture = await TestSpace.open({
        packages: [{ objects: { note: sharedNote }, tables: [] }],
    });
    const { database } = fixture;
    const moving = { id: ids.space, scope: ids.account, machine: fixture.machine.id, epoch: 1 };
    const steps = await SpaceTransfer.of(database, moving, UNKNOWN_MACHINE);
    // list the transfer table first, then the logged space tables in declaration order,
    //  leaving out the space's row from the universe
    expect(steps.space.tables.map((table) => table[TABLE].sqlName)).toEqual([
        "destack__stack__transfer",
        "destack__sync__scope",
        "destack__access__role",
        "destack__access__role_permission",
        "destack__access__relationship",
        "destack__access__invitation",
        "destack__object__call",
        "destack__event__segment",
        "destack__object__proof",
        "destack__object__installation",
        "destack__object__branch",
        "destack__object__branch_entry",
        "destack__object__branch_change",
        "destack__object__approval",
        "destack__object__change_request",
        "destack__stack__rollout",
        "destack__stack__installation_revision",
        "destack__stack__installation_key",
        "destack__stack__binding",
        "destack__stack__deployment",
        "destack__stack__capture",
        "destack__stack__instance",
        "destack__stack__build",
        "destack__stack__route",
        "destack__setting__setting",
        "destack__stack__database",
        "destack__object__desired_state",
        "destack__stack__bucket",
        "destack__stack__dependency",
        "destack__stack__https",
        "destack__stack__shared_note",
        "destack__account__identity_key",
    ]);
});

test("move the tables a space's main database applied in addition to the space database's, reading only the applied ones while a plan waits", async () => {
    // apply an installed package's pins to the space's provisioned main database
    await using fixture = await TestSpace.open();
    const { database } = fixture;
    const pinState = DatabaseState.of([pin]);
    const digest = await Digest.json(pinState, fixture.environment.crypto);
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(desiredState)
                .values({ scope: ids.space, digest, state: pinState }),
    );
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
    const moving = { id: ids.space, scope: ids.account, machine: fixture.machine.id, epoch: 1 };
    const states = await SpaceTransfer.states(database, ids.space);
    const moved = names(await SpaceTransfer.of(database, moving, UNKNOWN_MACHINE));
    const own = names(await SpaceTransfer.of(database, moving, UNKNOWN_MACHINE, []));

    // read only the applied states while the main database waits to apply its desired ones
    await database.transaction(
        async (transaction) =>
            await transaction.update(object.database.table).set({ appliedStates: [] }).where(main),
    );
    const waiting = await SpaceTransfer.states(database, ids.space);
    const pending = names(await SpaceTransfer.of(database, moving, UNKNOWN_MACHINE));

    // move the space's own tables and then the pins, a plan waiting or not
    expect({ states, moved, waiting, pending }).toEqual({
        states: [pinState],
        moved: [...own, "destack__stack__pin"],
        waiting: [],
        pending: own,
    });
});

test("stop the instances of a moving space running on its machine once its fence starts", async () => {
    // run an instance of the notes installation in the space
    await using space = await TestSpace.open();
    const release = await space.release(ids.package, "2026.9.0");
    const running = await runNotes(
        space.database,
        release,
        space.owner.subject,
        space.machine.environment,
    );
    const runtime = new TestRuntime("bun", [running]);

    // fence the moving space on the machine running it
    const moving = { id: ids.space, scope: ids.account, machine: space.machine.id, epoch: 1 };
    const transfer = await SpaceTransfer.of(space.database, moving, UNKNOWN_MACHINE);
    await transfer.fence(
        { providers: new ProviderIndex(space.machine.id, []), runtimes: [runtime] },
        space.machine.clock.now(),
    );
    expect(runtime.stopped).toEqual([running]);
});

test(
    "cancel a fenced transfer on its source as the space's owner, serving the space there again",
    async () => {
        // let the space's owner move it to the target
        const { source: fixture, targetId, universe } = await openMove();
        const { database } = fixture;
        const serving = await fixture.serve();
        const announcing = serving.store;
        const owner = serving.as(fixture.owner.subject).stepUp().client(stackPackage);
        const created = await owner.transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer", fixture.environment),
            requestId: RequestId.create(fixture.machine.environment),
            targetId: targetId,
        });
        await reconcile(announcing, "transfer", { id: created.id });

        // let the target have the source fence and stop as if its captures failed each time
        const moving = { spaceId: ids.space, id: created.id };
        await serving
            .as(targetId)
            .client(stackPackage)
            .transfer.fence({
                ...moving,
                requestId: RequestId.create(fixture.machine.environment),
            });

        // let the source follow the target's receipt, then conceal the transfer from its cancellation
        await fixture.settle(serving);
        const cancel = (caller: StackClient) =>
            caller.transfer.cancel({
                ...moving,
                requestId: RequestId.create(fixture.machine.environment),
            });
        const refused = await refusal(cancel(serving.as(targetId).client(stackPackage)));

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
            .where(eq(placement.table.id, ids.space));
        const [ended] = await database
            .select({
                completedAt: object.transfer.table.completedAt,
                outcome: object.transfer.table.outcome,
            })
            .from(object.transfer.table);
        const served = await database
            .select({ id: accountObject.space.table.id })
            .from(accountObject.space.table)
            .where(and(eq(accountObject.space.table.id, ids.space), accountObject.Space.served()));
        expect({
            refused,
            placement: placedAgain,
            outcome: [ended?.completedAt !== null, ended?.outcome],
            served,
            again: await refusal(cancel(owner)),
        }).toEqual({
            refused: ["NOT_FOUND", "NOT_FOUND", `transfer ${created.id} not found`],
            placement: { machine: fixture.machine.id, epoch: 2, target: null },
            outcome: [true, "cancelled"],
            served: [{ id: ids.space }],
            again: ["CONFLICT", "TRANSFER_ENDED", `transfer ${created.id} has ended`],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "move a space again to a machine it moved away from, removing the fenced rows of that move first",
    async () => {
        // move the space to the target and back, leaving the target the fenced rows of its move away
        const { source, target, targetId, universe } = await openMove();
        const sourceId = source.machine.id;
        const leaving = await source.serve();
        const away = await moveSpace(source, leaving, target, targetId);
        await leaving[Symbol.asyncDispose]();
        await away[Symbol.asyncDispose]();
        const back = await target.serve();
        await (await moveSpace(target, back, source, sourceId))[Symbol.asyncDispose]();
        await back[Symbol.asyncDispose]();

        // move it to the target again, which removes the fenced rows before it receives the space
        await moveSpace(source, await source.serve(), target, targetId);
        const transfers = await target.database
            .select({
                sourceId: object.transfer.table.sourceId,
                sourceEpoch: object.transfer.table.sourceEpoch,
                outcome: object.transfer.table.outcome,
            })
            .from(object.transfer.table)
            .orderBy(object.transfer.table.sourceEpoch);
        expect({ placement: await placementOf(universe), transfers }).toEqual({
            placement: { id: ids.space, scope: ids.account, machine: targetId, epoch: 4 },
            transfers: [
                { sourceId, sourceEpoch: 1, outcome: "succeeded" },
                { sourceId: targetId, sourceEpoch: 2, outcome: "succeeded" },
                { sourceId, sourceEpoch: 3, outcome: "succeeded" },
            ],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "move a space afresh after its cancelled move, never completing the old takeover",
    async () => {
        // add an enabled installation on the space's machine and an empty target
        const { source, target, targetId, universe } = await openMove();
        const now = source.machine.clock.now();
        await insertRollout(
            source.database,
            {
                id: ids.notes,
                scope: ids.space,
                packageId: ids.package,
                name: "notes",
                selection: { kind: "release", version: "2026.9.0" },
                installer: source.owner.subject,
            },
            now,
        );

        // register a key of the installation as the owner once the source serves the space
        const serving = await source.serve();
        const owner = createClient(
            stackPackage,
            serving.as(source.owner.subject).stepUp().endpoint(stackPackage),
            source.machine.environment,
        );
        const { crypto } = source.machine.environment;
        const pair = await crypto.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]);
        const { kty, crv, x } = await Jwk.export(pair.publicKey, crypto);
        const key = await owner.installationKey.create({
            spaceId: ids.space,
            parentId: ids.notes,
            requestId: RequestId.create(source.machine.environment),
            name: "ci",
            publicKey: PublicKey.parse({ kty, crv, x }),
        });

        // move the space to the target, which the owner cancels once the target took it over and before it places it
        const created = await owner.transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer", source.machine.environment),
            requestId: RequestId.create(source.machine.environment),
            targetId,
        });
        const receiving = await target.serve({
            role: "standby",
            publisher: cancelling((await target.options()).publisher, async () => {
                // cancel as the owner, and let the target see its placement no longer moves here
                await owner.transfer.cancel({
                    spaceId: ids.space,
                    id: created.id,
                    requestId: RequestId.create(source.machine.environment),
                });
                await target.database.log.until(
                    async () => (await placementRow(target.database))?.targetId === null,
                    FOREVER,
                );
            }),
            handoff: async () => target.handoff(serving, serving.as(targetId)),
        });
        const cancelled = await settleMove(
            source,
            serving,
            target,
            receiving,
            (placed) => placed?.machine === source.machine.id && placed.epoch === 2,
        );

        // ask a store of the target for a token as the installation, and the abandoned target for the installation as the owner
        const grant = async (served: TestObjectStore) => {
            // ask as the installation by an assertion its key signs
            const installation = principal.installation.reference(ids.space, ids.notes);
            const signing = { id: key.id, subject: installation, publicKey: key.publicKey };
            const assertion = await Assertion.sign(
                pair.privateKey,
                signing,
                PackageId.resource(objectPackage.id),
                source.machine.clock.now(),
                source.machine.environment,
            );

            return createClient(
                objectPackage,
                {
                    url: "http://space.test",
                    headers: { authorization: `Bearer ${assertion}` },
                    fetch: (request) => served.server(objectPackage).fetch(request),
                },
                source.machine.environment,
            ).installation.token({
                scope: ids.space,
                id: ids.notes,
                requestId: RequestId.create(source.machine.environment),
                audience: ids.package,
            });
        };
        const abandoned = [
            await refusal(grant(receiving)),
            await refusal(
                createClient(
                    stackPackage,
                    receiving.as(source.owner.id).endpoint(stackPackage),
                    source.machine.environment,
                ).rollout.get({ spaceId: ids.space, id: ids.notes }),
            ),
        ];

        // move the space afresh to the same target as the owner, and grant tokens as the target serves it placed there
        await owner.transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer", source.machine.environment),
            requestId: RequestId.create(source.machine.environment),
            targetId,
        });
        const moved = await settleMove(
            source,
            serving,
            target,
            receiving,
            (placed) => placed?.machine === targetId,
        );
        const received = await target.database
            .select({
                sourceEpoch: object.transfer.table.sourceEpoch,
                outcome: object.transfer.table.outcome,
            })
            .from(object.transfer.table)
            .orderBy(object.transfer.table.sourceEpoch);
        await receiving[Symbol.asyncDispose]();
        const token = await grant(await target.serve());
        expect({
            cancelled,
            abandoned,
            moved,
            placement: await placementOf(universe),
            transfers: received,
            granted: token.tokenType,
        }).toEqual({
            cancelled: [],
            abandoned: [
                ["UNAUTHORIZED", "ASSERTION_KEY_UNKNOWN", "assertion has no standing key"],
                ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${ids.space} not found`],
            ],
            moved: [],
            placement: { id: ids.space, scope: ids.account, machine: targetId, epoch: 3 },
            transfers: [
                { sourceEpoch: 1, outcome: "cancelled" },
                { sourceEpoch: 2, outcome: "succeeded" },
            ],
            granted: "Bearer",
        });
    },
    SCENARIO_MILLISECONDS,
);

test("sweep the blobs no snapshot reads while no snapshot is taken and no other sweep has the lease, then retire the expired snapshots", async () => {
    // serve the fixture's space on the machine that stores the snapshots of it and the home space
    const fixture = await TestSpace.open({});
    const served = serveSpace(await fixture.options()).store;
    const { database } = fixture;

    // add a taken snapshot of a resource of each space with one blob, and a blob none reads
    const store = fixture.store();
    const now = fixture.machine.clock.now();
    const write = (content: string) =>
        store.write(
            (async function* () {
                yield new TextEncoder().encode(content);
            })(),
        );
    const snapshotOf = async (
        space: Identifier<"space">,
        resource: string,
        blob: Digest | null,
    ) => {
        const id = schema
            .identifier("snapshot")
            .parse(object.snapshot.generateId(fixture.environment));
        await database.transaction(
            async (transaction) =>
                await transaction.insert(object.snapshot.table).values({
                    id,
                    spaceId: space,
                    resourceId: ResourceId.parse(resource),
                    recipient: (await Recipient.generate(fixture.environment)).key,
                    digest: blob,
                    takenAt: blob === null ? null : now,
                    blobs: blob === null ? [] : [blob],
                    createdAt: now,
                    updatedAt: now,
                }),
        );

        return id;
    };
    for (const [space, resource] of [
        [ids.space, "database-01996ab0-0000-7000-8000-0000000000f1"],
        [fixture.owner.home, "database-01996ab0-0000-7000-8000-0000000000f2"],
    ] as const) {
        await snapshotOf(space, resource, await write(resource));
    }
    await write("stray");

    // leave every blob while a snapshot is taken
    const taking = await snapshotOf(
        ids.space,
        "database-01996ab0-0000-7000-8000-0000000000f3",
        null,
    );
    await sweepSnapshots(at(served, now), store);
    const whileTaking = store.size;

    // leave every blob while another sweep has the lease
    const other = {
        ...SWEEP_LEASE,
        holder: "other",
        duration: 60_000,
        now: fixture.machine.clock.now(),
    };
    await Lease.acquire(database, other);
    const restoring = schema
        .identifier("restore")
        .parse(object.restore.generateId(fixture.environment));
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.restore.table).values({
                id: restoring,
                spaceId: ids.space,
                resourceId: ResourceId.parse("database-01996ab0-0000-7000-8000-0000000000f4"),
                sourceId: UNKNOWN_MACHINE,
                sourceEpoch: 1,
                createdAt: now,
                updatedAt: now,
            }),
    );
    const waits = [
        await reconcile(served, "snapshot", { id: taking }),
        await reconcile(served, "restore", { id: restoring }),
    ];
    await database.transaction(
        async (transaction) =>
            await transaction
                .delete(object.snapshot.table)
                .where(eq(object.snapshot.table.id, taking)),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .delete(object.restore.table)
                .where(eq(object.restore.table.id, restoring)),
    );
    await sweepSnapshots(at(served, now), store);
    const whileLeased = store.size;
    await Lease.release(database, other);

    // delete the stray blob, then the expired snapshots and their blobs two days later
    await sweepSnapshots(at(served, now), store);
    const swept = store.size;
    await sweepSnapshots(at(served, now + 2 * DAY_MILLISECONDS), store);
    expect({
        blobs: [whileTaking, whileLeased, swept, store.size],
        waits,
        snapshots: await database.select().from(object.snapshot.table),
    }).toEqual({ blobs: [3, 3, 2, 0], waits: [other.duration, other.duration], snapshots: [] });
});

/** Read a store as a sweep at a moment does, on a clock paused there. */
function at(
    store: Pick<ObjectStore, "database" | "mutate" | "environment">,
    time: number,
): Pick<ObjectStore, "database" | "mutate" | "clock" | "environment"> {
    const clock = new SimulatedClock(time);

    return {
        database: store.database,
        mutate: store.mutate.bind(store),
        clock,
        environment: { ...store.environment, clock },
    };
}

/** Run a cancellation once just before an uplink's first placement call. */
function cancelling(publisher: Uplink, cancel: () => Promise<unknown>): Uplink {
    let pending: (() => Promise<unknown>) | undefined = cancel;

    return {
        ...publisher,
        receive: async (mutation, signer) => {
            // cancel once before the first placement call
            const isPlacement = mutation.calls.some((call) => call.type.type === "placement");
            if (isPlacement && pending !== undefined) {
                const running = pending;
                pending = undefined;
                await running();
            }

            return publisher.receive(mutation, signer);
        },
    };
}

/** Open the space on its residency's machine and an empty target machine its owner enrolls. */
async function openMove() {
    // enroll the target as the owner, and host an empty database there next to the space's source
    const source = await TestSpace.open();
    const enrolled = await source.universe.enroll(source.owner, ids.account, TARGET_ORIGIN);
    const target = await TestSpace.empty({
        world: source,
        owner: source.owner,
        machine: enrolled,
    });

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
    serving: TestObjectStore,
    target: TestSpace,
    targetId: Identifier<"machine">,
): Promise<TestObjectStore> {
    // ask for the move as the space's owner on its source, once it follows the space's chain
    const isFollowed = async () =>
        (await Scope.read(Snapshot.live(source.database), [ids.account])).length > 0;
    await source.settle(serving);
    await source.simulation.settle(source.database.log.until(isFollowed, FOREVER));
    await serving
        .as(source.owner.subject)
        .stepUp()
        .client(stackPackage)
        .transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer", source.machine.environment),
            requestId: RequestId.create(source.machine.environment),
            targetId,
        });

    // receive it on the target as a standby, handed off by the source, until the universe places it there
    const receiving = await target.serve({
        role: "standby",
        handoff: async () => target.handoff(serving, serving.as(targetId)),
    });
    const refused = await settleMove(
        source,
        serving,
        target,
        receiving,
        (placed) => placed?.machine === targetId,
    );
    expect(refused).toEqual([]);

    return receiving;
}

/** Settle a move's source and target until its placement settles, listing the target's refusals. */
async function settleMove(
    source: TestSpace,
    serving: TestObjectStore,
    target: TestSpace,
    receiving: TestObjectStore,
    isSettled: (placed: Placement | undefined) => boolean,
    expected = 0,
): Promise<(readonly [string, string, string])[]> {
    // settle both machines in turn until the placement settles, past the failures a test expects retried
    let retries = expected;
    const universe = source.universe.storage.database;
    const positions = async () =>
        JSON.stringify(
            await Promise.all(
                [universe, source.database, target.database].map((database) =>
                    database.log.position(),
                ),
            ),
        );
    const refused: (readonly [string, string, string])[] = [];
    let before = await positions();
    while (!isSettled(await placementOf(universe))) {
        await source.settle(serving);
        refused.push(...(await TestSpace.refusals(target.settle(receiving))));
        const after = await positions();

        // advance past an expected retry of a failed step, else refuse a round that commits nothing
        if (after === before && retries > 0) {
            retries -= 1;
            await target.advance(
                receiving,
                target.simulation.clock.now() + FIRST_RETRY_MILLISECONDS,
            );
        } else if (after === before) {
            throw new Error("the move went idle before its placement settled");
        }
        before = after;
    }

    // let both machines' directories follow the new placement
    const head = await universe.log.position();
    await source.simulation.settle(source.machine.follow(head, FOREVER));
    await target.simulation.settle(target.machine.follow(head, FOREVER));

    return refused;
}

/** Read the space's placement row a database replicates, none while unplaced. */
async function placementRow(database: DatabaseConnection) {
    const [row] = await database
        .select()
        .from(placement.table)
        .where(eq(placement.table.id, ids.space));

    return row;
}

/** Read the space's placement in the universe, none while unplaced. */
async function placementOf(universe: DatabaseConnection): Promise<Placement | undefined> {
    const [placed] = await Placement.find(Snapshot.live(universe), [ids.space]);

    return placed;
}

/** List the SQL names of the tables a transfer moves, in order. */
function names(transfer: SpaceTransfer): string[] {
    return transfer.space.tables.map((table) => table[TABLE].sqlName);
}
