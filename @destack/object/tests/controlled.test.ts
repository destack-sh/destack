import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { Scope, RequestId } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, Caller, none, principal, relation } from "@destack/access";

import { defineDatabase, eq, Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";

import { defineObject, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the machines. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");

/** The machine the system provisions under the identifier it chose. */
const MACHINE = "machine-01996ab0-0000-7000-8000-000000000041";

/** The machine the space moves to. */
const TARGET = "machine-01996ab0-0000-7000-8000-000000000042";

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
        provision: method.create("provision", { visibility: "package" }),
        update: method.update("write", { fields: ["size"] }),
        delete: method.delete("write"),
    }),
);

/** The database with the machines, their access and the journal. */
const machineDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { machine }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("observe a controlled object without advancing its generation, finalize its deletion, and refuse a moved scope", async () => {
    // run in a world of its own
    const simulation = testSimulation(0);
    const world = simulation.environment("test");

    const storage = await TestDatabase.create(machineDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf([machine]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: {
            package: machine.package,
        },
        environment: world,
    });
    const at = async (time: number) => {
        await simulation.jump(time);

        return AccessContext.of(Caller.of(spacePrincipal(spaceId)), time);
    };
    const read = async () => {
        const [row] = await storage.database.select().from(machine.table);

        return row;
    };

    // create a machine as the system on behalf of its owner
    await store.mutate(await at(1_000), {
        id: RequestId.create(world),
        calls: [
            machine.call("provision", {
                ...machine.target({ scope: spaceId, id: MACHINE }),
                ownerId: users.frank,
                size: "small",
            }),
        ],
    });
    const created = present(await read(), "the machine");

    // observe it twice and preserve the transition time while the status stays
    const ready = { status: "false", reason: "Provisioning", message: "" } as const;
    await store.mutate(await at(2_000), {
        id: RequestId.create(world),
        calls: [
            machine.call("observe", {
                ...machine.target(created),
                observedGeneration: 1,
                conditions: { Ready: ready },
            }),
        ],
    });
    await store.mutate(await at(3_000), {
        id: RequestId.create(world),
        calls: [
            machine.call("observe", {
                ...machine.target(present(await read(), "the machine")),
                observedGeneration: 1,
                conditions: { Ready: { ...ready, message: "waiting for capacity" } },
                fields: { address: "10.0.0.1" },
            }),
        ],
    });
    const observed = present(await read(), "the machine");

    // delete it through its controller, requesting and finalizing inside an open transaction
    await store.mutate(await at(4_000), {
        id: RequestId.create(world),
        calls: [machine.call("delete", machine.target(observed))],
    });
    const requested = present(await read(), "the machine");
    const transaction = await Transaction.open(storage.database);
    await store.mutate(
        await at(5_000),
        {
            id: RequestId.create(world),
            calls: [machine.call("finalize", { spaceId, id: created.id })],
        },
        { transaction },
    );
    await transaction.commit();
    const finalized = await read();

    // refuse system work once the space moves to another machine
    await store.mutate(await at(6_000), {
        id: RequestId.create(world),
        calls: [
            machine.call("provision", {
                ...machine.in(spaceId),
                ownerId: users.frank,
                size: "large",
            }),
        ],
    });
    await Transaction.run(storage.database, (fencing) =>
        Scope.fence(fencing, spaceId, TARGET, 7_000),
    );
    const moved = store.mutate(await at(8_000), {
        id: RequestId.create(world),
        calls: [
            machine.call("observe", {
                ...machine.target(present(await read(), "the machine")),
                observedGeneration: 1,
                conditions: {},
            }),
        ],
    });
    expect(await refusal(moved)).toEqual([
        "MISDIRECTED_REQUEST",
        "SCOPE_MOVED",
        `${spaceId} moved to ${TARGET}`,
    ]);

    // observations leave the generation and the first transition time, deletion advances it
    expect([
        [created.id, created.generation, created.revision],
        [observed.generation, observed.revision, observed.address, observed.conditions],
        [requested.generation, requested.revision, requested.deletedAt],
        finalized,
    ]).toEqual([
        [MACHINE, 1, 1],
        [
            1,
            3,
            "10.0.0.1",
            {
                Ready: {
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
});

test("answer a repeated system request from the journal, provisioning once, and refuse other input under its identifier", async () => {
    // run in a world of its own
    const simulation = testSimulation(0);
    const world = simulation.environment("test");

    const storage = await TestDatabase.create(machineDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf([machine]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: machine.package },
        environment: world,
    });
    const at = async (time: number) => {
        await simulation.jump(time);

        return AccessContext.of(Caller.of(spacePrincipal(spaceId)), time);
    };

    // provision under a request twice, then with another size under it
    const requestId = await RequestId.derive(1_000, "machine small", world.crypto);
    const provision = async (size: string) =>
        store.mutate(await at(1_000), {
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
        other: ["CONFLICT", "REQUEST_REUSED", "request identifier has already been used"],
    });
});

/** Contacts a controller reconciles, each acting on its row as the contact principal it is. */
const agent = defineObject(
    {
        name: "contact",
        plural: "contacts",
        scope: space,
        represents: principal.contact,
        controlled: true,
        fields: { name: field.string() },
        permissions: { read: none(), write: none() },
    },
    (method) => ({ create: method.create("write") }),
);

test("let an object representing a principal control its row, and refuse it another's", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // create two contacts in the space
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { agent }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf([agent]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: agent.package },
        environment: world,
    });
    const mutate = (caller: AccessContext, call: ReturnType<typeof agent.call>) =>
        store.mutate(caller, { id: RequestId.create(world), calls: [call] });
    const create = async (name: string) => {
        const [created] = await mutate(
            AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()),
            agent.call("create", { spaceId, name }),
        );

        return schema.looseObject({ id: schema.string() }).parse(created).id;
    };
    const [first, second] = [await create("first"), await create("second")];

    // observe the first contact as itself, then as the second, which cannot see it
    const observe = (actor: string) =>
        mutate(
            AccessContext.of(
                Caller.of(principal.contact.reference(spaceId, actor)),
                world.clock.now(),
            ),
            agent.call("observe", {
                spaceId,
                id: first,
                observedGeneration: 1,
                conditions: { Ready: { status: "true", reason: "Running", message: "" } },
            }),
        );
    await observe(first);
    const refused = await refusal(observe(second));
    const [observed] = await storage.database
        .select()
        .from(agent.table)
        .where(eq(agent.table.id, schema.identifier("contact").parse(first)));

    expect({ observed: observed?.conditions["Ready"]?.observedGeneration, refused }).toEqual({
        observed: 1,
        refused: ["NOT_FOUND", "NOT_FOUND", `contact ${first} not found`],
    });
});
