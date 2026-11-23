import { worldTelemetry } from "../src/test/world.ts";
import { testSimulation, testIdentity, testEvents, TestObjectStore } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { PackageVersion } from "@destack/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { ObjectClient } from "../src/client/index.ts";
import {
    defineAction,
    defineObject,
    definePackage,
    field,
    me,
    selected,
    shareable,
    tablesOf,
} from "../src/index.ts";
import { implementPackage } from "../src/server/index.ts";
import { schema } from "@destack/schema";
import { spaceId } from "./fixture/device.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { principal } from "@destack/access";
import objectPackage from "../src/package.ts";
import { argument, plural, t } from "@destack/locale";

/** Jobs of a space, each named, open or taken, with who took it. */
const job = defineObject(
    {
        name: "job",
        plural: "jobs",
        title: t`${plural(argument("count"), { one: "Job", other: "Jobs" })}`,
        scope: space,
        fields: {
            name: field.string(schema.string()),
            status: field.enum(["open", "taken"]).default("open"),
            takenBy: field.subject(principal.user).optional(),
        },
        traits: [shareable({})],
    },
    (method) => ({
        update: method.update("write"),
        /** Take the job in the call's transaction, as the same action. */
        grab: method.mutation({ title: t`Grab`, permission: "write" }),
    }),
);

/** Take open jobs oneself, marking each by its name. */
const take = defineAction({
    name: "take",
    title: t`Take`,
    input: { jobs: selected(job, { status: "open" }) },
    call: ({ jobs }) => jobs.prepare("update", { status: "taken", takenBy: me, name: jobs.name }),
});

test("perform one action on a device, predicted and undone as one, and in a store's transaction", async () => {
    // serve the jobs of a space, whose grab performs the action in its transaction
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { job }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const jobs = definePackage({ ...testIdentity, objects: [job] });
    const grabbing = job.implement({
        methods: {
            grab: async (call) => {
                await call.perform(call.caller, take, { values: { jobs: call.reference() } });

                return call.target;
            },
        },
    });
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(jobs, { objects: [grabbing] })],
            chain: async () => undefined,
            environment: world,
            origin: { package: PackageVersion.of(jobs) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());

    // follow them on a device of Alice's
    const local = await TestDatabase.create(ObjectClient.tables({ job }), {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        environment: world,
        database: local.database,
        objects: { job },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        remote: served.as(users.alice).connect(jobs, { scope: spaceId }),
    });
    onTestFinished(() => client.close());
    client.queryType(job).findMany().subscribe();
    const stopping = new AbortController();
    const loops = [client.follow(stopping.signal), client.push(stopping.signal)];
    const ids: string[] = [];
    for (const name of ["Paint", "Sweep", "Mend"]) {
        const created = client.mutate(job).create({ name });
        ids.push((await created.predicted).id);
        await created.confirmed;
    }
    const state = async () =>
        (await client.queryType(job).findMany({ orderBy: { name: "asc" } })).map((row) => [
            row.name,
            row.status,
            row.takenById,
        ]);

    // take the first two on the device, then the third in the store's transaction
    const references = ids.map((id) => job.policy.reference(spaceId, id));
    await client.undoStack.close();
    const taken = client.perform(take, { values: { jobs: references.slice(0, 2) } });
    await taken.predicted;
    const predicted = await state();
    await taken.confirmed;
    const refused = await refusal(
        () => client.perform(take, { values: { jobs: references.slice(0, 1) } }).predicted,
    );
    await client.undoStack.undo().confirmed;
    const undone = await state();
    const [, , third] = ids;
    await client.call(job, "grab", { id: third ?? "" }).confirmed;
    await expect.poll(async () => (await state())[0]).toEqual(["Mend", "taken", users.alice]);
    stopping.abort();
    await Promise.all(loops);

    expect({ predicted, undone, refused, stored: await state() }).toEqual({
        // both taken at once, before the store confirmed
        predicted: [
            ["Mend", "open", null],
            ["Paint", "taken", users.alice],
            ["Sweep", "taken", users.alice],
        ],
        // both open again after one undo
        undone: [
            ["Mend", "open", null],
            ["Paint", "open", null],
            ["Sweep", "open", null],
        ],
        refused: [
            "PRECONDITION_FAILED",
            "CALL_BLOCKED",
            `cannot call 'job.update': it does not apply to job '${ids[0]}'`,
        ],
        // the third taken by the same action on the store
        stored: [
            ["Mend", "taken", users.alice],
            ["Paint", "open", null],
            ["Sweep", "open", null],
        ],
    });
    expect(worldTelemetry(client.environment).exceptions()).toEqual([]);
});
