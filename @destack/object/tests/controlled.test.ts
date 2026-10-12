import { Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test, TestClock } from "@destack/test";
import { none, principal, relation, Caller } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";

import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { RequestId } from "@destack/service/request";
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
    permissions: { read: relation("owner"), write: relation("owner"), provision: none() },
}).methods((method) => ({
    create: method.create("write", { fields: ["size"] }),
    provision: method.create("provision", { isInternal: true }),
    update: method.update("write", { fields: ["size"] }),
    delete: method.delete("write"),
}));

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
        const clock = new TestClock(0);
        const store = new ObjectStore({
            principal: spacePrincipal(spaceId),
            objects: { machine },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: machine.package,
                service: "test",
            },
            clock,
        });
        const at = (time: number) => {
            clock.set(time);

            return Caller.of(spacePrincipal(spaceId), time);
        };
        const read = async () => {
            const [row] = await storage.database.select().from(machine.table);

            return row;
        };

        // create a machine as the system on behalf of its owner
        await store.mutate(at(1_000), {
            id: RequestId.create(clock.now()),
            calls: [
                machine.call("provision", {
                    ...machine.target({ scope: spaceId, id: MACHINE }),
                    ownerId: "global:user-1",
                    size: "small",
                }),
            ],
        });
        const created = present(await read(), "the machine");

        // observe it twice and keep the transition time while the status stays
        const ready = { status: "false", reason: "Provisioning", message: "" } as const;
        await store.mutate(at(2_000), {
            id: RequestId.create(clock.now()),
            calls: [
                machine.call("observe", {
                    ...machine.target(created),
                    observedGeneration: 1,
                    conditions: { ready },
                }),
            ],
        });
        await store.mutate(at(3_000), {
            id: RequestId.create(clock.now()),
            calls: [
                machine.call("observe", {
                    ...machine.target(present(await read(), "the machine")),
                    observedGeneration: 1,
                    conditions: { ready: { ...ready, message: "waiting for capacity" } },
                    fields: { address: "10.0.0.1" },
                }),
            ],
        });
        const observed = present(await read(), "the machine");

        // delete it through its controller, requesting and finalizing inside an open transaction
        await store.mutate(at(4_000), {
            id: RequestId.create(clock.now()),
            calls: [machine.call("delete", machine.target(observed))],
        });
        const requested = present(await read(), "the machine");
        const transaction = await Transaction.open(storage.database);
        await store.mutate(
            at(5_000),
            {
                id: RequestId.create(clock.now()),
                calls: [machine.call("finalize", { spaceId, id: created.id })],
            },
            { transaction },
        );
        await transaction.commit();
        const finalized = await read();

        // refuse system work once the space moves to another machine
        await store.mutate(at(6_000), {
            id: RequestId.create(clock.now()),
            calls: [
                machine.call("provision", {
                    ...machine.in(spaceId),
                    ownerId: "global:user-1",
                    size: "large",
                }),
            ],
        });
        await Transaction.run(storage.database, (transaction) =>
            Scope.fence(transaction, spaceId, "host-2", 7_000),
        );
        const moved = store.mutate(at(8_000), {
            id: RequestId.create(clock.now()),
            calls: [
                machine.call("observe", {
                    ...machine.target(present(await read(), "the machine")),
                    observedGeneration: 1,
                    conditions: {},
                }),
            ],
        });
        await expect(moved).rejects.toMatchObject({
            code: "MISDIRECTED_REQUEST",
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

test("answer a repeated system request from the journal, provisioning once, and refuse other input under its identifier", async () => {
    const storage = await TestDatabase.create("sqlite", machineDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const clock = new TestClock(0);
    const store = new ObjectStore({
        principal: spacePrincipal(spaceId),
        objects: { machine },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: machine.package, service: "test" },
        clock,
    });
    const at = (time: number) => {
        clock.set(time);

        return Caller.of(spacePrincipal(spaceId), time);
    };

    // provision under a request twice, then with another size under it
    const requestId = await RequestId.derive(1_000, "machine small");
    const provision = (size: string) =>
        store.mutate(at(1_000), {
            id: requestId,
            calls: [
                machine.call("provision", {
                    ...machine.in(spaceId),
                    ownerId: "global:user-1",
                    size,
                }),
            ],
        });
    const [first] = await provision("small");
    const [repeated] = await provision("small");
    const other = await refusal(provision("large"));

    // keep one machine, answer the repeat with the recorded one, and refuse the other input
    expect({
        machines: await storage.database.select().from(machine.table),
        repeated,
        other,
    }).toEqual({
        machines: [first],
        repeated: first,
        other: ["CONFLICT", "request identifier has already been used"],
    });
});
