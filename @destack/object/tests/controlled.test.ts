import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation, Scope } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { defineJournal, Journal } from "@destack/service/database";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer, SystemCall } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The space holding the machines. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");

/** The machine the system provisions under the identifier it chose. */
const MACHINE = "machine-01996ab0-0000-7000-8000-000000000041";

/** Machines a controller provisions. */
const machine = defineObject({
    name: "machine",
    plural: "machines",
    scope: space,
    controlled: true,
    fields: {
        owner: field.reference(principal.user).caller(),
        size: field.string(schema.string().min(1)),
        address: field.string().optional(),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: {
        create: method.create("write", { fields: ["size"] }),
        provision: method.create(null, { isSystem: true }),
        update: method.update("write", { fields: ["size"] }),
        delete: method.delete("write"),
    },
});

/** Replayable machine requests. */
const journal = defineJournal("journal");

/** The database holding the machines, their access and the journal. */
const machineDatabase = defineDatabase({
    name: "main",
    tables: [...auditOutboxTables, journal, ...machine.tables],
});

test.each(TEST_DIALECTS)(
    "observe a controlled object without advancing its generation, finalize its deletion, and refuse a moved scope, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, machineDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const server = new ObjectServer({
            objects: { machine },
            database: storage.database,
            context: () => ({ subjects: [], now: 0, attributes: {} }),
            journal: new Journal(journal),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: machine.package,
                service: "test",
            }),
        });
        const read = async () => {
            const [row] = await storage.database.select().from(machine.table);

            return row;
        };

        // create a machine as the system on behalf of its owner
        await server.executeAsSystem(
            machine,
            "provision",
            [{ scope: spaceId, id: MACHINE, input: { owner: "global:user-1", size: "small" } }],
            1_000,
        );
        const created = (await read())!;

        // observe it twice, keeping the transition time while the status holds
        const ready = { status: "false", reason: "Provisioning", message: "" } as const;
        await server.executeAsSystem(
            machine,
            "observe",
            [SystemCall.of(created, { observedGeneration: 1, conditions: { ready } })],
            2_000,
        );
        await server.executeAsSystem(
            machine,
            "observe",
            [
                SystemCall.of((await read())!, {
                    observedGeneration: 1,
                    conditions: { ready: { ...ready, message: "waiting for capacity" } },
                    fields: { address: "10.0.0.1" },
                }),
            ],
            3_000,
        );
        const observed = (await read())!;

        // delete it through its controller: request, then finalize inside an open transaction
        await server.executeAsSystem(machine, "delete", [SystemCall.of(observed)], 4_000);
        const requested = (await read())!;
        await storage.database.transaction(async (transaction) => {
            await server.invoke(
                transaction,
                spaceId,
                machine,
                "finalize",
                { id: created.id },
                5_000,
            );
        });
        const finalized = await read();

        // refuse system work once the space moves to another holder
        await server.executeAsSystem(
            machine,
            "provision",
            [{ scope: spaceId, input: { owner: "global:user-1", size: "large" } }],
            6_000,
        );
        await Scope.fence(storage.database, spaceId, "host-2", 7_000);
        const moved = server.executeAsSystem(
            machine,
            "observe",
            [SystemCall.of((await read())!, { observedGeneration: 1, conditions: {} })],
            8_000,
        );
        await expect(moved).rejects.toMatchObject({
            code: "MOVED",
            message: `${spaceId} moves to host-2`,
        });

        // observations keep the generation and the first transition time, deletion advances it
        expect([
            [created.id, created.generation, created.observedGeneration, created.revision],
            [
                observed.generation,
                observed.observedGeneration,
                observed.revision,
                observed.address,
                observed.conditions,
            ],
            [requested.generation, requested.revision, requested.deletionRequestedAt],
            finalized,
        ]).toEqual([
            [MACHINE, 1, 0, 1],
            [
                1,
                1,
                3,
                "10.0.0.1",
                {
                    ready: {
                        status: "false",
                        reason: "Provisioning",
                        message: "waiting for capacity",
                        observedGeneration: 1,
                        lastTransitionAt: 2_000,
                    },
                },
            ],
            [2, 4, 4_000],
            undefined,
        ]);
    },
);
