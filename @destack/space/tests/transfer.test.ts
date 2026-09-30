import { spaceTables } from "../src/stack/index.ts";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";
import { type DirectoryDatabase, ZONE_SCOPE, zoneTable } from "@destack/directory";
import { and, defineTable, eq, integer, TABLE, text } from "@destack/db";
import { DatabaseKind, defineDatabase } from "@destack/db/declare";
import { sqliteConnector, sqliteProvider, type SqliteDatabase } from "@destack/db/sqlite";
import { TestDatabase } from "@destack/db/test";
import { defineObject, field } from "@destack/object";
import { Plan } from "@destack/resource";
import { identifier } from "@destack/schema";
import { digest } from "@destack/schema/json";
import { ControlLoop } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import { serveObjects, ZoneTransfer, type ZoneRelay } from "../src/server/index.ts";
import * as object from "../src/object/index.ts";
import {
    copyAccount,
    ids,
    openGlobal,
    openSpace,
    reconcile,
    serveSpace,
    spaceOptions,
} from "./fixture/space.ts";

/** The host receiving the space. */
const TARGET = identifier("host").parse("host-01996ab0-0000-7000-8000-00000000000b");

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
    scope: object.space,
    fields: { body: field.string() },
    permissions: ["read"],
});

/** The database resource keeping the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

test("receive a space on the host it moves to: follow it, have the source fence, copy its databases, drop the copy of one deleted meanwhile, refuse an export without its end, and take it over", async () => {
    // keep the space on the fixture's region, and an empty PostgreSQL database on the target host
    const source = await openSpace();
    const target = await TestDatabase.create("postgresql", spaceTables, { isMigrated: true });
    onTestFinished(() => target.close());
    await copyAccount(target.database);
    const global = await openGlobal();
    const directory = global.directory as DirectoryDatabase;
    await directory.place({ id: ids.space, scope: ids.account, cell: ids.region, epoch: 1 });
    await directory.publish(TARGET, ids.account, "http://target.test/");

    // keep an applied notes database with two notes on the source for an installation's binding
    const files = await mkdtemp(join(tmpdir(), "destack-space-transfer-"));
    onTestFinished(() => rm(files, { recursive: true, force: true }));
    const sources = sqliteProvider(pathToFileURL(`${files}/source/`));
    const targets = sqliteProvider(pathToFileURL(`${files}/target/`));
    const now = Date.now();
    const declared = {
        id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-00000000000c"),
        scope: ids.space,
        name: "notes",
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "2026.9.0",
        definitionName: "notes",
        spec: { tier: "zonal" },
        createdAt: now,
        updatedAt: now,
    };
    const desired = [notes.state()];
    const provision = await sources.provision(
        DatabaseKind.record({ ...declared, reference: null }),
    );
    const resource = {
        ...declared,
        reference: provision.reference,
        providerCode: "sqlite",
        status: { state: { digest: await digest(desired) } },
    };
    await source.insert(object.resource.table).values(resource);

    // keep an unbound scratch database the source deletes after its first copy
    const scratchId = identifier("resource").parse("resource-01996ab0-0000-7000-8000-00000000000e");
    const scratchDeclared = { ...declared, id: scratchId, name: "scratch" };
    const scratchProvision = await sources.provision(
        DatabaseKind.record({ ...scratchDeclared, reference: null }),
    );
    const scratch = {
        ...scratchDeclared,
        reference: scratchProvision.reference,
        providerCode: "sqlite",
        status: { state: { digest: await digest([]) } },
    };
    await source.insert(object.resource.table).values(scratch);
    const scratchRecord = DatabaseKind.record(scratch);
    await sources.apply(
        scratchRecord,
        [],
        await Plan.digest(await sources.plan(scratchRecord, [])),
    );
    await source.insert(object.installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });
    await source.insert(object.binding.table).values({
        id: identifier("binding").parse("binding-01996ab0-0000-7000-8000-00000000000d"),
        scope: ids.space,
        installationId: ids.notes,
        packageId: ids.package,
        name: "notes",
        target: declared.id,
        state: desired[0]!,
        createdAt: now,
        updatedAt: now,
    });
    const provisioned = DatabaseKind.record(resource);
    await sources.apply(
        provisioned,
        desired,
        await Plan.digest(await sources.plan(provisioned, desired)),
    );
    const written = (await sqliteConnector.connect(
        {
            resource: resource.id,
            kind: resource.kind,
            provider: "sqlite",
            reference: resource.reference!,
        },
        notes,
    )) as SqliteDatabase;
    await written.insert(note).values([
        { id: "n1", scope: ids.space, body: "first", rank: 1 },
        { id: "n2", scope: ids.space, body: "second", rank: 2 },
    ]);
    await written.close();

    // serve the space and let its owner create a transfer to the target
    const client = await serveSpace(source, global, {
        providers: [sources],
    });
    const created = await client("owner", { isAuthenticatedNow: true }).transfer.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        target: TARGET,
    });
    const announcing = serveObjects(await spaceOptions(source, { global, providers: [sources] }));
    await reconcile(announcing, "transfer", { id: created.id });

    // receive it on the target until the directory delegates it there, cutting the end off the first export
    const errors: unknown[] = [];
    let isCut = true;
    const relay = async (cell: string): Promise<ZoneRelay> => {
        expect(cell).toBe(ids.region);
        const relayed = client(TARGET).relay;

        return {
            watch: relayed.watch,
            fence: relayed.fence,
            export: async (input, options) => {
                // delete the scratch database on the source once its first copy was exported
                const records = await relayed.export(input, options);
                if (input.resourceId === scratchId) {
                    const exported = [];
                    for await (const record of records) {
                        exported.push(record);
                    }
                    await source
                        .delete(object.resource.table)
                        .where(eq(object.resource.table.id, scratchId));

                    return (async function* () {
                        yield* exported;
                    })();
                }

                // cut the end off the first export of the notes database
                const isCutting = isCut;
                isCut = false;

                return (async function* () {
                    for await (const record of records) {
                        if (!isCutting || !("end" in record)) {
                            yield record;
                        }
                    }
                })();
            },
        } as ZoneRelay;
    };
    const receiving = serveObjects(
        await spaceOptions(target.database, {
            global: { ...global, relay },
            cell: { hostId: TARGET },
            providers: [targets],
        }),
    );

    // copy the zone moving here, as the target copies the zones it may read
    const moving = (await directory.locate(ids.space))!;
    await target.database.insert(zoneTable).values({
        id: moving.id,
        scope: ZONE_SCOPE,
        parent: moving.scope,
        cell: moving.cell,
        epoch: moving.epoch,
        target: TARGET,
    });
    const incoming = receiving.controllers().filter((each) => each.name === "zone");
    const stopping = new AbortController();
    const received = new ControlLoop(target.database, incoming, {
        report: (_controller, _key, error) => errors.push(error),
        retry: { initialInterval: 1 },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await received;
    });
    await directory.database.log.until(
        async () => (await directory.locate(ids.space))?.cell === TARGET,
        AbortSignal.timeout(8000),
    );
    stopping.abort();
    await received;

    // leave the directory alone on the fenced source
    await reconcile(announcing, "transfer", { id: created.id });

    // keep the space, its notes and its records on the target, placed there at the next epoch
    const [copied, ...others] = await target.database.select().from(object.resource.table);
    const read = (await sqliteConnector.connect(
        {
            resource: copied!.id,
            kind: copied!.kind,
            provider: "sqlite",
            reference: copied!.reference!,
        },
        notes,
    )) as SqliteDatabase;
    const [record] = await target.database.select().from(object.transfer.table);
    const [part] = await target.database.select().from(object.resourceTransfer.table);
    const served = await target.database
        .select({ id: object.space.table.id })
        .from(object.space.table)
        .where(and(eq(object.space.table.id, ids.space), object.SpaceCell.served()));
    expect({
        errors,
        zone: await directory.locate(ids.space),
        notes: await read.select().from(note).orderBy(note.id),
        completed: [record!.completedAt !== null, record!.source, record!.target],
        part: [part!.sourceReference, part!.targetReference, part!.targetProviderCode],
        reference: copied!.reference,
        others,
        databases: (await readdir(join(files, "target", ids.space))).filter((name) =>
            name.endsWith(".db"),
        ),
        served,
    }).toEqual({
        errors: [new Error(`the export of ${declared.id} stopped before its end`)],
        zone: { id: ids.space, scope: ids.account, cell: TARGET, epoch: 2 },
        notes: [
            { id: "n1", scope: ids.space, body: "first", rank: 1 },
            { id: "n2", scope: ids.space, body: "second", rank: 2 },
        ],
        completed: [true, ids.region, TARGET],
        part: [provision.reference, copied!.reference, "sqlite"],
        reference: pathToFileURL(`${files}/target/${ids.space}/${declared.id}.db`).href,
        others: [],
        databases: [`${declared.id}.db`],
        served: [{ id: ids.space }],
    });
    await read.close();
});

test("refuse a transfer without a recent sign-in, to an unknown host, or to a host of another account", async () => {
    // publish one host of another account beside the fixture's region
    const database = await openSpace();
    const global = await openGlobal();
    const directory = global.directory as DirectoryDatabase;
    await directory.place({ id: ids.space, scope: ids.account, cell: ids.region, epoch: 1 });
    const foreign = identifier("host").parse("host-01996ab0-0000-7000-8000-00000000000c");
    const other = identifier("account").parse("account-01996ab0-0000-7000-8000-00000000000d");
    await directory.publish(foreign, other, "http://foreign.test/");
    const client = await serveSpace(database, global);

    // refuse each transfer with its reason
    const refused = (target: string, isAuthenticatedNow: boolean) =>
        client("owner", { isAuthenticatedNow })
            .transfer.create({ spaceId: ids.space, requestId: RequestId.create(), target })
            .then(
                () => "created",
                (error: { code: string; message: string }) => [error.code, error.message],
            );
    expect([
        await refused(TARGET, false),
        await refused(TARGET, true),
        await refused(foreign, true),
    ]).toEqual([
        ["INSUFFICIENT_AUTHENTICATION", "authenticate again at the required assurance"],
        ["BAD_REQUEST", `${TARGET} is no host of the space's account nor a region`],
        ["BAD_REQUEST", `${foreign} is no host of the space's account nor a region`],
    ]);
});

test("move every table of the cell database keeping rows in spaces, leaving out the cell's own, copied and unlogged tables", async () => {
    // declare a cell database with another package's space-scoped notes beside the space package's tables
    const database = await openSpace([noteObject.table]);
    const moving = { id: ids.space, scope: ids.account, cell: ids.region, epoch: 1 };
    const steps = ZoneTransfer.of(database, moving, TARGET);
    // keep the space row and transfers first, then every logged space-scoped table in declaration order
    expect(steps.tables.map((table) => table[TABLE].sqlName)).toEqual([
        "destack__space__space",
        "destack__space__transfer",
        "destack__space__journal",
        "destack__space__network_policy",
        "destack__sync__scope",
        "destack__access__role",
        "destack__access__role_permission",
        "destack__access__relationship",
        "destack__access__proposal",
        "destack__space__network_policy_version",
        "destack__space__package_policy",
        "destack__space__package_policy_version",
        "destack__space__binding",
        "destack__space__capture",
        "destack__space__resource",
        "destack__space__restoration",
        "destack__space__snapshot",
        "destack__space__deployment",
        "destack__space__installation",
        "destack__space__installation_revision",
        "destack__space__instance",
        "destack__space__run",
        "destack__space__schedule",
        "destack__space__resource_transfer",
        "destack__setting__setting",
        "destack__space__shared_note",
    ]);
});
