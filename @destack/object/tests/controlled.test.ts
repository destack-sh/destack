import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";

import { defineObject, field } from "../src/index.ts";
import { ObjectServer, SystemCall } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";

/** The space with the machines. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");

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
    methods: (method) => ({
        create: method.create("write", { fields: ["size"] }),
        provision: method.create(null, { isSystem: true }),
        update: method.update("write", { fields: ["size"] }),
        delete: method.delete("write"),
    }),
});

/** The database with the machines, their access and the journal. */
const machineDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...machine.tables],
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
            callKey: testCallKey,
            origin: {
                package: machine.package,
                service: "test",
            },
        });
        const read = async () => {
            const [row] = await storage.database.select().from(machine.table);

            return row;
        };

        // create a machine as the system on behalf of its owner
        await server.executeAsSystem(
            machine,
            "provision",
            [{ scope: spaceId, id: MACHINE, input: { ownerId: "global:user-1", size: "small" } }],
            1_000,
        );
        const created = present(await read(), "the machine");

        // observe it twice and keep the transition time while the status stays
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
                SystemCall.of(present(await read(), "the machine"), {
                    observedGeneration: 1,
                    conditions: { ready: { ...ready, message: "waiting for capacity" } },
                    fields: { address: "10.0.0.1" },
                }),
            ],
            3_000,
        );
        const observed = present(await read(), "the machine");

        // delete it through its controller, requesting and finalizing inside an open transaction
        await server.executeAsSystem(machine, "delete", [SystemCall.of(observed)], 4_000);
        const requested = present(await read(), "the machine");
        await storage.database.transaction(async (transaction) => {
            await server
                .invoke(machine, { database: transaction, scope: spaceId, now: 5_000 })
                .finalize({ id: created.id });
        });
        const finalized = await read();

        // refuse system work once the space moves to another cell
        await server.executeAsSystem(
            machine,
            "provision",
            [{ scope: spaceId, input: { ownerId: "global:user-1", size: "large" } }],
            6_000,
        );
        await Scope.fence(storage.database, spaceId, "host-2", 7_000);
        const moved = server.executeAsSystem(
            machine,
            "observe",
            [
                SystemCall.of(present(await read(), "the machine"), {
                    observedGeneration: 1,
                    conditions: {},
                }),
            ],
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
