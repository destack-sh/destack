import { users } from "./fixture/user.ts";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test, TestClock } from "@destack/test";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";

import { defineDatabase, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Clock, present, schema } from "@destack/schema";

import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";

/** The space with the machines. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");

/** The machine the system provisions under the identifier it chose. */
const MACHINE = "machine-01996ab0-0000-7000-8000-000000000041";

/** Machines a controller provisions. */
const machine = defineObject(
    {
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
    },
    (method) => ({
        create: method.create("write", { fields: ["size"] }),
        provision: method.create("provision", { isInternal: true }),
        update: method.update("write", { fields: ["size"] }),
        delete: method.delete("write"),
    }),
);

/** The database with the machines, their access and the journal. */
const machineDatabase = defineDatabase({
    name: "main",
    tables: [...callTables, ...machine.tables],
});

test.each(TEST_DIALECTS)(
    "observe a controlled object without advancing its generation, finalize its deletion, and refuse a moved scope, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, machineDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const clock = new TestClock(0);
        const store = new ObjectStore({
            chain: async () => undefined,
            principal: spacePrincipal(spaceId),
            servers: serversOf([machine]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: machine.package,
            },
            clock,
        });
        const at = (time: number) => {
            clock.set(time);

            return AccessContext.of(Caller.of(spacePrincipal(spaceId)), time);
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
                    ownerId: users.frank,
                    size: "small",
                }),
            ],
        });
        const created = present(await read(), "the machine");

        // observe it twice and hold the transition time while the status stays
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
                    ownerId: users.frank,
                    size: "large",
                }),
            ],
        });
        await Transaction.run(storage.database, (fencing) =>
            Scope.fence(fencing, spaceId, "host-2", 7_000),
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

        // observations leave the generation and the first transition time, deletion advances it
        expect([
            [created.id, created.generation, created.observedGeneration, created.revision],
            [
                observed.generation,
                observed.observedGeneration,
                observed.revision,
                observed.address,
                observed.conditions,
            ],
            [requested.generation, requested.revision, requested.deletedAt],
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
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf([machine]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: machine.package },
        clock,
    });
    const at = (time: number) => {
        clock.set(time);

        return AccessContext.of(Caller.of(spacePrincipal(spaceId)), time);
    };

    // provision under a request twice, then with another size under it
    const requestId = await RequestId.derive(1_000, "machine small");
    const provision = (size: string) =>
        store.mutate(at(1_000), {
            id: requestId,
            calls: [
                machine.call("provision", {
                    ...machine.in(spaceId),
                    ownerId: users.frank,
                    size,
                }),
            ],
        });
    const [first] = await provision("small");
    const [repeated] = await provision("small");
    const other = await refusal(provision("large"));

    // create one machine, answer the repeat with the recorded one, and refuse the other input
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

/** Installations a controller reconciles, each acting on its own row as the principal it is. */
const agent = defineObject(
    {
        name: "installation",
        plural: "installations",
        scope: space,
        represents: principal.installation,
        controlled: true,
        fields: { name: field.string() },
        permissions: { read: none(), write: none() },
    },
    (method) => ({ create: method.create("write") }),
);

test("let an object representing a principal control its own row, and refuse it another's", async () => {
    // create two installations in the space
    const storage = await TestDatabase.create("sqlite", [...callTables, ...agent.tables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf([agent]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: agent.package },
        clock: Clock.system,
    });
    const mutate = (caller: AccessContext, call: ReturnType<typeof agent.call>) =>
        store.mutate(caller, { id: RequestId.create(Date.now()), calls: [call] });
    const create = async (name: string) => {
        const [created] = await mutate(
            AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()),
            agent.call("create", { spaceId, name }),
        );

        return schema.looseObject({ id: schema.string() }).parse(created).id;
    };
    const [first, second] = [await create("first"), await create("second")];

    // observe the first installation as itself, then as the second, which cannot see it
    const observe = (actor: string) =>
        mutate(
            AccessContext.of(
                Caller.of(principal.installation.reference(spaceId, actor)),
                Date.now(),
            ),
            agent.call("observe", { spaceId, id: first, observedGeneration: 1, conditions: {} }),
        );
    await observe(first);
    const refused = await refusal(observe(second));
    const [observed] = await storage.database.select().from(agent.table);

    expect({ observed: observed?.observedGeneration, refused }).toEqual({
        observed: 1,
        refused: ["NOT_FOUND", `no installation ${first}`],
    });
});
