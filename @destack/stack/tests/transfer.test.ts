import { PackageId } from "@destack/package";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { PublicKey, Recipient } from "@destack/identity";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Identity, Placement, placement } from "@destack/account/object";
import { IdentityKeystore } from "@destack/account/identity";
import { TestDirectory } from "@destack/account/test";
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
    Transaction,
} from "@destack/db";
import { databaseProvider } from "@destack/db";
import { BunSqliteDatabaseHost, bunSqliteConnector } from "@destack/db/bun";
import { SqliteDatabase } from "@destack/db/sqlite";
import { TEST_DIALECTS } from "@destack/db/test";
import { defineObject, desiredState, field } from "@destack/object";
import { Plan, ResourceId } from "@destack/resource";
import { reconciliation } from "@destack/runtime/test";
import { aligned, Clock, Digest, Identifier, schema, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { ControlLoop } from "@destack/runtime/control";
import { RequestId } from "@destack/runtime/request";
import { Assertion } from "@destack/runtime/authentication";
import {
    ProviderIndex,
    serveSpace,
    sweepSnapshots,
    SpaceTransfer,
    TransferController,
    type Handoff,
} from "../src/server/index.ts";
import { fixtureBuild, spaceDatabase } from "../src/test/index.ts";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { createLocalJWKSet, exportJWK, generateKeyPair, jwtVerify } from "jose";
import * as object from "../src/object/index.ts";
import * as accountObject from "@destack/account/object";
import {
    replicateAccount,
    ids,
    openUniverse,
    reconcile,
    TestRuntime,
    RunFixture,
    StackFixture,
} from "../src/test/index.ts";
import { Scope, type Uplink } from "@destack/sync";
import { principal } from "@destack/access";
import { machine } from "@destack/account/object";
import { createClient } from "@destack/runtime/client";
import stackPackage from "../src/package.ts";
import type { StackClient } from "../src/client/index.ts";

/** How long one move between two machines waits for its next step: twenty times a move. */
const MOVE_MILLISECONDS = 20_000;

/** How long a test moving a space between two machines runs at most, beyond its longest wait. */
const SCENARIO_MILLISECONDS = 30_000;

/** The host receiving the space. */
const TARGET = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-00000000000b");

/** The notes an installation keeps in the space's database resource. */
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

/** A space's notes as objects of another package sharing the space database. */
const noteObject = defineObject({
    name: "shared-note",
    plural: "sharedNotes",
    scope: accountObject.space,
    fields: { body: field.string() },
    permissions: ["read"],
}).methods();

/** A space's database keeping shared notes beside the space's tables. */
const sharedNotesDatabase = defineDatabase({
    name: "space",
    tables: [
        ...spaceDatabase.tables.filter((table) => !spaceDatabase.replicates(table)),
        noteObject.table,
    ],
    replicated: spaceDatabase.tables.filter((table) => spaceDatabase.replicates(table)),
});

/** The database resource keeping the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

/** Open a replica of the notes database through its connector, refusing another dialect. */
async function openNotes(id: ResourceId, reference: string): Promise<SqliteDatabase> {
    const connection = await bunSqliteConnector.connect(
        { resource: id, kind: "database", provider: "sqlite", reference },
        notes,
    );
    if (!(connection instanceof SqliteDatabase)) {
        throw new TypeError(`the notes database ${id} is no SQLite database`);
    }

    return connection;
}

test.each(TEST_DIALECTS)(
    "receive a moving space with its rows, databases and keys, then remove the source's replica on %s",
    async (dialect) => {
        // keep the space on the fixture's cloud machine and an empty database on the target host
        const sourceMachine = await StackFixture.open();
        const source = sourceMachine.database;
        await replicateTarget(source);
        const built = await sourceMachine.recordBuild(await fixtureBuild(ids.package, "2026.9.0"));
        const target = await StackFixture.empty({ dialect });
        onTestFinished(() => target[Symbol.asyncDispose]());
        await replicateAccount(target.database);
        const opened = await openUniverse();
        const { identities } = opened;
        const placed = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
        await TestDirectory.place(opened.universe, placed);
        await TestDirectory.publish(opened.universe, TARGET, ids.account, "http://target.test/");

        // generate the space's keys under the source machine's keyring distinct from the target's
        const sourceKeyring = await LocalKeyring.open(new MemoryKeychain(), "source");
        const targetKeyring = await LocalKeyring.open(new MemoryKeychain(), "target");
        await new IdentityKeystore(sourceKeyring, identities, Clock.system).generate(
            source,
            placed.id,
        );

        // keep an applied notes database with two notes on the source for an installation's binding
        const files = await mkdtemp(join(tmpdir(), "destack-space-transfer-"));
        onTestFinished(() => rm(files, { recursive: true, force: true }));
        const sources = databaseProvider(
            new BunSqliteDatabaseHost(pathToFileURL(`${files}/source/`)),
            object.database,
        );
        const targets = databaseProvider(
            new BunSqliteDatabaseHost(pathToFileURL(`${files}/target/`)),
            object.database,
        );
        const now = Date.now();
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
        const digest = await Digest.json(state);
        const provision = await sources.provision.provision(
            DatabaseKind.record({ ...declared, reference: null }),
        );
        const resource = {
            ...declared,
            reference: provision.reference,
            provider: "sqlite",
            states: [digest],
            appliedState: await Digest.json([digest]),
        };
        await source.insert(desiredState).values({ scope: ids.space, digest, state });
        await source.insert(object.database.table).values(resource);

        // keep an unbound scratch database the source deletes before its fence
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
            appliedState: await Digest.json([]),
        };
        await source.insert(object.database.table).values(scratch);
        const scratchRecord = DatabaseKind.record(scratch);
        await sources.reconcile.apply(
            scratchRecord,
            [],
            await Plan.digest(await sources.reconcile.plan(scratchRecord, [])),
        );
        await source.insert(object.installation.table).values({
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            role: "application",
            name: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            createdAt: now,
            updatedAt: now,
        });
        await source.insert(object.binding.table).values({
            id: schema.identifier("binding").parse("binding-01996ab0-0000-7000-8000-00000000000d"),
            scope: ids.space,
            installationId: ids.notes,
            packageId: ids.package,
            name: "notes",
            target: declared.id,
            state,
            createdAt: now,
            updatedAt: now,
        });
        const provisioned = DatabaseKind.record(resource);
        await sources.reconcile.apply(
            provisioned,
            desired,
            await Plan.digest(await sources.reconcile.plan(provisioned, desired)),
        );
        const written = await openNotes(resource.id, resource.reference);
        await written.insert(note).values([
            { id: "n1", scope: ids.space, body: "first", rank: 1 },
            { id: "n2", scope: ids.space, body: "second", rank: 2 },
        ]);
        await written.close();

        // serve the space and let its owner create a transfer to the target
        const { server, store: announcing } = await sourceMachine.server({
            ...opened,
            providers: [sources],
            keystore: new IdentityKeystore(sourceKeyring, identities, Clock.system),
        });
        const client = (caller: string, connection?: { readonly isAuthenticatedNow?: boolean }) =>
            createClient(stackPackage, StackFixture.endpoint(server, caller, connection));
        const created = await client(ids.owner, { isAuthenticatedNow: true }).transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer"),
            requestId: RequestId.create(Date.now()),
            targetId: TARGET,
        });
        await reconcile(announcing, "transfer", { id: created.id });

        // receive it on the target until the universe places it there and cut the end off the first capture
        const errors: unknown[] = [];
        let isCut = true;
        const handoff = async (from: string): Promise<Handoff> => {
            expect(from).toBe(ids.cloud);
            const handed = StackFixture.handoff(server, TARGET);

            return {
                ...handed,
                fence: async (moving) => {
                    // delete the scratch database on the source before the fence while the target follows it
                    await source
                        .delete(object.database.table)
                        .where(eq(object.database.table.id, scratchId));

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
        const receiver = await target.options({
            ...opened,
            handoff,
            machine: TARGET,
            providers: [targets],
            keystore: new IdentityKeystore(targetKeyring, identities, Clock.system),
        });
        const receiving = serveSpace(receiver);

        // replicate the space moving here as the target replicas the spaces it may read
        const [moving] = await opened.universe
            .select()
            .from(placement.table)
            .where(eq(placement.table.id, ids.space));
        await target.database.insert(placement.table).values({
            ...present(moving, "the moving placement"),
            targetId: TARGET,
            transferId: created.id,
        });
        await target.database
            .insert(accountObject.space.table)
            .values(await source.select().from(accountObject.space.table));
        const restoring = present(
            receiving.store.controllers().find((controller) => controller.name === "restore"),
            "the restore controller",
        );
        const incoming = [transferControllerOf(receiving), restoring];
        const stopping = new AbortController();
        const received = new ControlLoop(target.database, incoming, {
            clock: Clock.system,
            report: (_controller, _key, error) => errors.push(error),
            retry: { initialInterval: 1 },
        }).run(stopping.signal);
        onTestFinished(async () => {
            stopping.abort();
            await received;
        });
        await opened.universe.log.until(
            async () => (await placementOf(opened.universe))?.machine === TARGET,
            AbortSignal.timeout(MOVE_MILLISECONDS),
        );
        stopping.abort();
        await received;

        // replicate the target's placement to the source as its replicas follow the universe
        await source
            .update(placement.table)
            .set({ machineId: TARGET, epoch: 2, targetId: null, transferId: null })
            .where(eq(placement.table.id, ids.space));

        // remove the source's replica and sweep its store a day later
        await reconcile(announcing, "transfer", { id: created.id });
        const sourceStore = sourceMachine.store();
        const blobs = sourceStore.size;
        const later = Date.now() + 2 * 24 * 60 * 60 * 1000;
        await sweepSnapshots(announcing, sourceStore, later);
        const removed = {
            databases: await source.select().from(object.database.table),
            snapshots: await source.select().from(object.snapshot.table),
            files: (await readdir(join(files, "source", ids.space))).filter((name) =>
                name.endsWith(".db"),
            ),
            blobs: [blobs > 0, sourceStore.size],
        };

        // keep the space, its notes and its records on the target
        await target.database
            .update(placement.table)
            .set({ machineId: TARGET, epoch: 2, targetId: null, transferId: null })
            .where(eq(placement.table.id, ids.space));
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
        );
        const record = aligned(await target.database.select().from(object.transfer.table), 0);
        const [recorded] = await target.database
            .select({ manifest: object.build.table.manifest })
            .from(object.build.table)
            .where(eq(object.build.table.id, built.build));
        const manifest = present(recorded?.manifest, "the moved build's manifest");
        const served = await target.database
            .select({ id: accountObject.space.table.id })
            .from(accountObject.space.table)
            .where(and(eq(accountObject.space.table.id, ids.space), accountObject.Space.served()));
        const signed = await new IdentityKeystore(targetKeyring, identities, Clock.system).sign(
            target.database,
            ids.space,
            {
                sub: ids.space,
            },
        );
        expect({
            errors,
            removed,
            placement: await placementOf(opened.universe),
            notes: await read.select().from(note).orderBy(note.id),
            completed: [record.completedAt !== null, record.sourceId, record.targetId],
            moved: await (await target.builds()).packages.contains(manifest),
            reference: replicated.reference,
            kept: kept.map((entry) => [entry.name, entry.provider]),
            databases: (await readdir(join(files, "target", ids.space))).filter((name) =>
                name.endsWith(".db"),
            ),
            served,
            signer: (
                await jwtVerify(
                    signed,
                    createLocalJWKSet(
                        await Identity.keys(Snapshot.live(opened.universe), ids.space),
                    ),
                )
            ).payload.sub,
        }).toEqual({
            // retry the unavailable capture without reporting it, inside the unavailable window
            errors: [],
            // keep the scratch database's file
            removed: {
                databases: [],
                snapshots: [],
                files: [`${scratchId}.db`],
                blobs: [true, 0],
            },
            placement: { id: ids.space, scope: ids.account, machine: TARGET, epoch: 2 },
            notes: [
                { id: "n1", scope: ids.space, body: "first", rank: 1 },
                { id: "n2", scope: ids.space, body: "second", rank: 2 },
            ],
            completed: [true, ids.cloud, TARGET],
            // replicate the space's build into the target's build store
            moved: true,
            reference: pathToFileURL(`${files}/target/${ids.space}/${declared.id}.db`).href,
            // keep the notes database and the space's unprovisioned main database
            kept: [
                ["main", null],
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
    // replicate one machine of another account beside the fixture's cloud machine
    const fixture = await StackFixture.open();
    const opened = await openUniverse();
    await TestDirectory.place(opened.universe, {
        id: ids.space,
        scope: ids.account,
        machine: ids.cloud,
        epoch: 1,
    });
    const foreign = schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-00000000000c");
    const other = schema
        .identifier("account")
        .parse("account-01996ab0-0000-7000-8000-00000000000d");
    const now = Date.now();
    await fixture.database.insert(machine.table).values({
        id: foreign,
        scope: other,
        name: "foreign",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
        createdAt: now,
        updatedAt: now,
    });
    const client = await fixture.serve(opened);

    // refuse each transfer with its reason
    const refused = (target: string, isAuthenticatedNow: boolean) =>
        refusal(
            client(ids.owner, { isAuthenticatedNow }).transfer.create({
                spaceId: ids.space,
                id: Identifier.create("transfer"),
                requestId: RequestId.create(Date.now()),
                targetId: target,
            }),
        );
    expect([
        await refused(TARGET, false),
        await refused(TARGET, true),
        await refused(foreign, true),
    ]).toEqual([
        ["INSUFFICIENT_AUTHENTICATION", "authentication assurance is too low: authenticate again"],
        [
            "BAD_REQUEST",
            `${TARGET} is no standing machine of the space's account nor of its residency`,
        ],
        [
            "BAD_REQUEST",
            `${foreign} is no standing machine of the space's account nor of its residency`,
        ],
    ]);
});

test("move every table of the space database keeping rows in spaces, leaving out the machine's own, replicated and unlogged tables", async () => {
    // declare a space database with another package's space-scoped notes beside the space's tables
    const fixture = await StackFixture.open({ database: sharedNotesDatabase });
    const { database } = fixture;
    const moving = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
    const steps = SpaceTransfer.of(database, moving, TARGET);
    // keep the transfers ahead of every logged space-scoped table in declaration order, the space's row arriving from the universe instead
    expect(steps.space.tables.map((table) => table[TABLE].sqlName)).toEqual([
        "destack__stack__transfer",
        "destack__stack__installation",
        "destack__sync__scope",
        "destack__access__role",
        "destack__access__role_permission",
        "destack__access__relationship",
        "destack__access__invitation",
        "destack__stack__installation_revision",
        "destack__stack__installation_key",
        "destack__stack__proof",
        "destack__stack__declaration",
        "destack__stack__binding",
        "destack__stack__deployment",
        "destack__stack__capture",
        "destack__stack__instance",
        "destack__stack__run",
        "destack__stack__run_outcome",
        "destack__event__segment",
        "destack__stack__build",
        "destack__stack__trigger",
        "destack__stack__route",
        "destack__stack__branch",
        "destack__stack__branch_entry",
        "destack__stack__branch_change",
        "destack__stack__database",
        "destack__object__desired_state",
        "destack__stack__bucket",
        "destack__stack__dependency",
        "destack__stack__https",
        "destack__setting__setting",
        "destack__object__call",
        "destack__account__identity_key",
        "destack__stack__shared_note",
    ]);
});

test("stop the instances of a moving space running on its machine once its fence starts", async () => {
    // keep a running instance of the notes installation in the space
    await using runs = await RunFixture.open("sqlite");
    const [running] = await runs.database
        .select({ id: object.instance.table.id })
        .from(object.instance.table);
    const runtime = new TestRuntime("bun", [present(running, "the running instance").id]);

    // fence the moving space on the machine running it
    const moving = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
    await SpaceTransfer.of(runs.database, moving, TARGET).fence(
        { providers: new ProviderIndex(ids.cloud, []), runtimes: [runtime] },
        Date.now(),
    );
    expect(runtime.stopped).toEqual([present(running, "the running instance").id]);
});

test.each(TEST_DIALECTS)(
    "cancel a fenced transfer on its source as the space's owner, serving the space there again, on %s",
    async (dialect) => {
        // let the space's owner move it to the target
        const fixture = await StackFixture.open({ dialect });
        const { database } = fixture;
        await replicateTarget(database);
        const opened = await openUniverse();
        const { identities } = opened;
        const placed = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
        await TestDirectory.place(opened.universe, placed);
        await TestDirectory.publish(opened.universe, TARGET, ids.account, "http://target.test/");
        const keystore = new IdentityKeystore(
            await LocalKeyring.open(new MemoryKeychain(), "source"),
            identities,
            Clock.system,
        );
        const { server, store: announcing } = await fixture.server({ ...opened, keystore });
        const client = (caller: string, connection?: { readonly isAuthenticatedNow?: boolean }) =>
            createClient(stackPackage, StackFixture.endpoint(server, caller, connection));
        const owner = client(ids.owner, { isAuthenticatedNow: true });
        const created = await owner.transfer.create({
            spaceId: ids.space,
            id: Identifier.create("transfer"),
            requestId: RequestId.create(Date.now()),
            targetId: TARGET,
        });
        await reconcile(announcing, "transfer", { id: created.id });

        // let the target have the source fence and stop as if its captures kept failing
        const moving = { spaceId: ids.space, id: created.id };
        await client(TARGET).transfer.fence({ ...moving, requestId: RequestId.create(Date.now()) });

        // conceal the transfer from the target's cancellation and cancel as the owner
        const cancel = (caller: StackClient) =>
            caller.transfer.cancel({ ...moving, requestId: RequestId.create(Date.now()) });
        const refused = await refusal(cancel(client(TARGET)));
        await cancel(owner);

        // serve the space placed at the next epoch with no move
        const [placedAgain] = await opened.universe
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
            refused: ["NOT_FOUND", `no transfer ${created.id}`],
            placement: { machine: ids.cloud, epoch: 2, target: null },
            outcome: [true, "cancelled"],
            served: [{ id: ids.space }],
            again: ["CONFLICT", `transfer ${created.id} has ended`],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "refuse a move to a machine keeping the fenced rows of an earlier move away, leaving the space on its source",
    async () => {
        // keep the space's rows on the target host fenced as a move away from it left them
        const keptMachine = await StackFixture.open();
        const kept = keptMachine.database;
        await Transaction.run(kept, (transaction) =>
            Scope.fence(transaction, ids.space, ids.cloud, Date.now()),
        );
        await kept
            .update(placement.table)
            .set({ targetId: TARGET, transferId: Identifier.create("transfer") })
            .where(eq(placement.table.id, ids.space));

        // move the space from the cloud machine back to the target host
        const opened = await openUniverse();
        const { identities } = opened;
        const placed = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
        await TestDirectory.place(opened.universe, placed, {
            target: TARGET,
            transferId: Identifier.create("transfer"),
        });

        // refuse receiving it over the stale rows
        const receiver = await keptMachine.options({
            ...opened,
            machine: TARGET,
            keystore: new IdentityKeystore(
                await LocalKeyring.open(new MemoryKeychain(), "target"),
                identities,
                Clock.system,
            ),
        });
        const transfers = transferControllerOf(serveSpace(receiver));
        const refused = await transfers
            .reconcile(ids.space, reconciliation(AbortSignal.timeout(MOVE_MILLISECONDS)))
            .catch((error: unknown) => error);
        expect({ refused, placement: await placementOf(opened.universe) }).toEqual({
            refused: new CallError("CONFLICT", {
                message: `${ids.space} keeps rows here from an earlier move, which this move would serve stale`,
            }),
            placement: placed,
        });
    },
    SCENARIO_MILLISECONDS,
);

test.each(TEST_DIALECTS)(
    "move a space afresh after its cancelled move, never completing the old takeover, on %s",
    async (dialect) => {
        // keep the space with its keys and an enabled installation on the cloud machine and an empty target
        const sourceMachine = await StackFixture.open({ dialect });
        const source = sourceMachine.database;
        await replicateTarget(source);
        const target = await StackFixture.empty({ dialect });
        onTestFinished(() => target[Symbol.asyncDispose]());
        await replicateAccount(target.database);
        await target.database
            .insert(accountObject.space.table)
            .values(await source.select().from(accountObject.space.table));
        const opened = await openUniverse();
        const { identities } = opened;
        const placed = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
        await TestDirectory.place(opened.universe, placed);
        await TestDirectory.publish(opened.universe, TARGET, ids.account, "http://target.test/");
        const sourceKeystore = new IdentityKeystore(
            await LocalKeyring.open(new MemoryKeychain(), "source"),
            identities,
            Clock.system,
        );
        await sourceKeystore.generate(source, placed.id);
        const now = Date.now();
        await source.insert(object.installation.table).values({
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            role: "application",
            name: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            createdAt: now,
            updatedAt: now,
        });

        // register a key of the installation as the owner
        const { server, store: announcing } = await sourceMachine.server({
            ...opened,
            keystore: sourceKeystore,
        });
        const owner = createClient(
            stackPackage,
            StackFixture.endpoint(server, ids.owner, { isAuthenticatedNow: true }),
        );
        const pair = await generateKeyPair("Ed25519");
        const { kty, crv, x } = await exportJWK(pair.publicKey);
        const key = await owner.installationKey.create({
            spaceId: ids.space,
            parentId: ids.notes,
            requestId: RequestId.create(Date.now()),
            name: "ci",
            publicKey: PublicKey.parse({ kty, crv, x }),
        });

        // move the space from its current placement and replica that to the target, remembering the transfer
        let moved: Identifier<"transfer"> | undefined;
        const move = async () => {
            await replicatePlacement(opened.universe, source);
            const created = await owner.transfer.create({
                spaceId: ids.space,
                id: Identifier.create("transfer"),
                requestId: RequestId.create(Date.now()),
                targetId: TARGET,
            });
            moved = created.id;
            await reconcile(announcing, "transfer", { id: created.id });
            await replicatePlacement(opened.universe, target.database);
        };

        // receive it on the target while the owner cancels once the target took it over and before it places the space
        const targetKeystore = new IdentityKeystore(
            await LocalKeyring.open(new MemoryKeychain(), "target"),
            identities,
            Clock.system,
        );
        const receiver = await target.options({
            ...opened,
            publisher: cancelling(await target.universe(opened.universe, TARGET), () =>
                owner.transfer.cancel({
                    spaceId: ids.space,
                    id: present(moved, "the moving transfer"),
                    requestId: RequestId.create(Date.now()),
                }),
            ),
            handoff: async () => StackFixture.handoff(server, TARGET),
            machine: TARGET,
            keystore: targetKeystore,
        });
        const receiving = serveSpace(receiver);
        const transfers = transferControllerOf(receiving);
        const receive = () =>
            transfers.reconcile(ids.space, reconciliation(AbortSignal.timeout(MOVE_MILLISECONDS)));

        // restore the space's resources on the target as its restore controller does
        const restoring = present(
            receiving.store.controllers().find((controller) => controller.name === "restore"),
            "the restore controller",
        );
        const errors: unknown[] = [];
        const stopping = new AbortController();
        const restored = new ControlLoop(target.database, [restoring], {
            clock: Clock.system,
            report: (_controller, _key, error) => errors.push(error),
            retry: { initialInterval: 1 },
        }).run(stopping.signal);
        onTestFinished(async () => {
            stopping.abort();
            await restored;
        });
        await move();
        const cancelled = await refusal(receive());

        // serve the target without the controllers the test runs by hand and refuse calls into its fenced replica
        const { server: targetServer } = await target.server({
            ...opened,
            keystore: targetKeystore,
            machine: TARGET,
            isControlled: false,
        });
        const served = (caller: string) =>
            createClient(stackPackage, StackFixture.endpoint(targetServer, caller));
        const grant = async () => {
            // ask as the installation by an assertion its key signs
            const installation = principal.installation.reference(ids.space, ids.notes);
            const signing = { id: key.id, subject: installation, publicKey: key.publicKey };
            const assertion = await Assertion.sign(
                pair.privateKey,
                signing,
                PackageId.resource(stackPackage.id),
                Date.now(),
            );

            return createClient(stackPackage, {
                url: "http://space.test",
                headers: { authorization: `Bearer ${assertion}` },
                fetch: (request) => targetServer.fetch(request),
            }).installation.token({
                spaceId: ids.space,
                id: ids.notes,
                requestId: RequestId.create(Date.now()),
                audience: ids.package,
            });
        };
        const fenced = [
            await refusal(grant()),
            await refusal(
                served(ids.owner).installation.get({ spaceId: ids.space, id: ids.notes }),
            ),
        ];

        // move the space afresh once the owner retries the same target and grant tokens there
        await move();
        await receive();
        const received = await target.database
            .select({
                sourceEpoch: object.transfer.table.sourceEpoch,
                outcome: object.transfer.table.outcome,
            })
            .from(object.transfer.table)
            .orderBy(object.transfer.table.sourceEpoch);
        const token = await grant();
        expect({
            errors,
            cancelled,
            fenced,
            placement: await placementOf(opened.universe),
            transfers: received,
            granted: token.tokenType,
        }).toEqual({
            errors: [],
            cancelled: ["NOT_FOUND", `no placement ${ids.space}`],
            fenced: [
                ["MISDIRECTED_REQUEST", `${ids.space} moves to ${TARGET}`],
                ["MISDIRECTED_REQUEST", `${ids.space} moves to ${TARGET}`],
            ],
            placement: { ...placed, machine: TARGET, epoch: 3 },
            transfers: [
                { sourceEpoch: 1, outcome: "cancelled" },
                { sourceEpoch: 2, outcome: "succeeded" },
            ],
            granted: "Bearer",
        });
    },
    SCENARIO_MILLISECONDS,
);

test.each(TEST_DIALECTS)(
    "retire the expired snapshots of every space a machine serves as each space, then sweep the blobs they read on %s",
    async (dialect) => {
        // serve the fixture's space on the machine keeping the snapshots of it and the home space
        const fixture = await StackFixture.open({ dialect });
        const served = serveSpace(await fixture.options()).store;

        // keep a taken snapshot of a resource of each space with one blob in the machine's store
        const store = fixture.store();
        const now = Date.now();
        for (const [space, resource] of [
            [ids.space, "database-01996ab0-0000-7000-8000-0000000000f1"],
            [ids.home, "database-01996ab0-0000-7000-8000-0000000000f2"],
        ] as const) {
            const blob = await store.write(
                (async function* () {
                    yield new TextEncoder().encode(resource);
                })(),
            );
            await fixture.database.insert(object.snapshot.table).values({
                id: schema.identifier("snapshot").parse(object.snapshot.generateId()),
                spaceId: space,
                resourceId: ResourceId.parse(resource),
                recipient: (await Recipient.generate()).key,
                digest: blob,
                takenAt: now,
                blobs: [blob],
                createdAt: now,
                updatedAt: now,
            });
        }

        // sweep the store two days later
        const blobs = store.size;
        await sweepSnapshots(served, store, now + 2 * 24 * 60 * 60 * 1000);
        expect({
            blobs: [blobs, store.size],
            snapshots: await fixture.database.select().from(object.snapshot.table),
        }).toEqual({ blobs: [2, 0], snapshots: [] });
    },
);

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

            await publisher.receive(mutation, signer);
        },
    };
}

/** Replicate the target host into a machine's database as one of the account's machines. */
async function replicateTarget(database: DatabaseConnection): Promise<void> {
    const now = Date.now();
    await database.insert(machine.table).values({
        id: TARGET,
        scope: ids.account,
        name: "target",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
        createdAt: now,
        updatedAt: now,
    });
}

/** Replicate the space's placement as the universe has it into a machine's database. */
async function replicatePlacement(
    universe: DatabaseConnection,
    database: DatabaseConnection,
): Promise<void> {
    const [row] = await universe
        .select()
        .from(placement.table)
        .where(eq(placement.table.id, ids.space));
    await database.delete(placement.table).where(eq(placement.table.id, ids.space));
    await database.insert(placement.table).values(present(row, `the placement of ${ids.space}`));
}

/** Find the controller receiving the spaces moving to the machine a served space runs on. */
function transferControllerOf(served: ReturnType<typeof serveSpace>): TransferController {
    const [stack] = served.packages;

    return present(
        stack?.controllers?.find((controller) => controller instanceof TransferController),
        "the transfer controller",
    );
}

/** Read the space's placement in the universe, none while unplaced. */
async function placementOf(universe: DatabaseConnection): Promise<Placement | undefined> {
    const [placed] = await Placement.find(Snapshot.live(universe), [ids.space]);

    return placed;
}
