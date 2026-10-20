import { anyone, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { STACKS } from "@destack/account/test";
import { bucket, type DatabaseConnection, defineDatabase, lease, Table } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { VirtualClock } from "@destack/environment/simulation";
import { defineObject, field, type ObjectType } from "@destack/object";
import { defineServer, ObjectStore } from "@destack/object/server";
import { callTables, testEvents } from "@destack/object/test";
import { PackageHandle } from "@destack/runtime";
import { ControlLoop } from "@destack/runtime/control";
import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { type Identifier, schema } from "@destack/schema";
import { type Call, type Subject, Scope } from "@destack/sync";
import { onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import type {
    RunContext,
    Step,
    Workflow,
    WorkflowImplementation,
    WorkflowServing,
} from "../src/index.ts";
import { approval, changeRequest } from "../src/approval/index.ts";
import workflowPackage from "../src/package.ts";
import workflowServer from "../src/server.ts";

/** The person starting the runs. */
export const alice = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000100"),
);

/** A person who started no run. */
export const bob = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000101"),
);

/** The installation serving the notes and running the workflows. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000201");

/** The time the scenarios start at: the current minute, as replicated access uses real time. */
export const START = Math.floor(Date.now() / 60_000) * 60_000;

/** Notes their owners write, and publish under a new title, which no role lets anyone else do. */
export const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: space,
        fields: { owner: field.subject().caller(), title: field.string() },
        permissions: {
            read: relation("owner"),
            write: relation("owner"),
            approve: relation("owner"),
            publish: relation("owner"),
        },
        attachments: [approval.attach({ by: "approve" }), changeRequest.attach({ by: "approve" })],
    },
    (method) => ({
        publish: method.mutation({
            permission: "publish",
            input: schema.object({ title: schema.string() }),
        }),
    }),
);

/** Notes as the fixture serves them, publishing under the given title. */
const servedNote = note.implement({
    methods: { publish: (call) => call.update({ title: call.input.title }) },
});

/** The workflow package as the fixtures serve it: approvals, change requests and change runs. */
const PACKAGE = workflowServer.definition(undefined);

/** A space serving notes, the workflow package and a workflow, on a virtual clock. */
export interface WorkflowFixture {
    /** The store serving the notes and the workflow. */
    readonly store: ObjectStore;
    /** The clock the store and the controller run on. */
    readonly clock: VirtualClock;
    /** The space the notes and runs live in. */
    readonly spaceId: Identifier<"space">;
    /** The workflow as the store serves it, its webhooks among it. */
    readonly served: WorkflowImplementation;
    /** Start a run as the store, acting as alice unless told otherwise. */
    readonly start: (call: (spaceId: Identifier<"space">) => Call) => Promise<unknown>;
    /** Call as alice in the space. */
    readonly asAlice: (calls: readonly Call[]) => Promise<readonly unknown[]>;
    /** Call as bob in the space. */
    readonly asBob: (calls: readonly Call[]) => Promise<readonly unknown[]>;
    /** Read the workflow's runs, oldest first. */
    readonly runs: () => Promise<readonly Record<string, unknown>[]>;
    /** Wait until the runs meet a condition, moving the clock to each next timer between looks. */
    readonly until: (
        condition: (runs: readonly Record<string, unknown>[]) => boolean,
    ) => Promise<readonly Record<string, unknown>[]>;
}

/** Serve notes and a workflow in alice's space, advancing runs on a virtual clock. */
export async function openWorkflow<Input, Output>(
    workflow: Workflow<Input, Output>,
    code: (step: Step, input: Input, context: RunContext) => Promise<Output>,
    serving: WorkflowServing = {},
    objects: readonly ObjectType[] = [],
): Promise<WorkflowFixture> {
    return openServed(workflow, workflow.implement(code, serving), [], objects);
}

/** A further workflow a fixture serves, which the main workflow's runs invoke. */
export interface InvokedServing {
    /** The workflow. */
    readonly workflow: Pick<Workflow, "name" | "objects">;
    /** The workflow as the store serves it. */
    readonly served: WorkflowImplementation;
}

/** Serve notes and served workflows in a space alice owns, advancing runs on a virtual clock. */
export async function openServed<Input, Output>(
    workflow: Workflow<Input, Output>,
    served: WorkflowImplementation,
    invoked: readonly InvokedServing[] = [],
    objects: readonly ObjectType[] = [],
): Promise<WorkflowFixture> {
    // migrate the notes, the runs and their steps, and the workflow package beside the journal, leases, buckets and access
    const tables = [
        ...callTables,
        ...space.tables,
        ...note.tables,
        ...objects.flatMap((type) => type.tables),
        ...[workflow, ...invoked.map((each) => each.workflow)].flatMap((each) =>
            Object.values(each.objects).flatMap((type) => type.tables),
        ),
        ...Object.values(workflowPackage.objects).flatMap((type) => type.tables),
        lease,
        bucket,
    ];
    const storage = await TestDatabase.create("sqlite", defineDatabase({ name: "main", tables }), {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const spaceId = await openSpace(storage.database);

    // serve the notes, the workflow package and the workflow as the space's installation
    const clock = new VirtualClock(START);
    const handle = PackageHandle.of(note.package, {
        objects: {
            note,
            ...Object.fromEntries(objects.map((type) => [type.name, type])),
            ...workflowPackage.objects,
            ...workflow.objects,
            ...Object.fromEntries(invoked.flatMap((each) => Object.entries(each.workflow.objects))),
        },
    });
    const servings = [served, ...invoked.map((each) => each.served)];
    const controllers = [
        ...(PACKAGE.controllers ?? []),
        ...servings.flatMap((serving) => serving.controllers),
    ];
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: principal.installation.reference(spaceId, INSTALLATION),
        servers: [
            defineServer(handle, {
                objects: [
                    servedNote,
                    ...objects,
                    ...(PACKAGE.objects ?? []),
                    ...servings.flatMap((serving) => serving.objects),
                ],
                controllers,
            }),
        ],
        policies: [account, space],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: note.package },
    });

    // grant the installation, as alice and bob each consent on first use, every permission of the served types
    const servedTypes = [
        note,
        ...objects,
        ...Object.values(workflowPackage.objects),
        ...[workflow, ...invoked.map((each) => each.workflow)].flatMap((each) =>
            Object.values(each.objects),
        ),
    ];
    const consented = servedTypes.flatMap((type) =>
        Object.keys(type.policy.definition.permissions).map((name) => ({
            ...type.policy.permission(name),
            scope: spaceId,
        })),
    );
    const installation = principal.installation.reference(spaceId, INSTALLATION);
    const access = new AccessFixture(storage.database);
    for (const user of [alice, bob]) {
        await access.replicateGrant(user, installation, consented, START);
    }

    // run the workflows' controllers until the test ends, failing the test on a reconciliation error
    const names = [workflow.name, ...invoked.map((each) => each.workflow.name), "change"];
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const loop = new ControlLoop(
        store.database,
        [
            ...store
                .controllers()
                .filter((each) => names.some((name) => each.name.startsWith(name))),
            ...controllers.map((build) => build(store)),
        ],
        { clock, report: (_controller, _key, error) => void failures.push(error) },
    );
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
        if (failures.length > 0) {
            throw failures[0];
        }
    });

    // read the runs, and wait for a condition over them
    const runs = async () =>
        (await store.database.select().from(workflow.run.table)).toSorted((left, right) =>
            schema.string().parse(left["id"]).localeCompare(schema.string().parse(right["id"])),
        );
    const until = async (condition: (rows: readonly Record<string, unknown>[]) => boolean) => {
        for (let look = 0; look < 1000; look++) {
            // look once the controllers settled, failing on their errors
            await loop.idle();
            if (failures.length > 0) {
                throw failures[0];
            }
            const rows = await runs();
            if (condition(rows)) {
                return rows;
            }

            // move the clock to the next timer, failing once none is left
            const next = clock.next;
            if (next === undefined) {
                break;
            }
            clock.set(next);
        }
        const states = (await runs()).map(
            (row) =>
                `${JSON.stringify(row["state"])}@${JSON.stringify(row["wakeAt"])} ${JSON.stringify(row["failure"])}`,
        );
        throw new Error(`the runs did not reach the condition: ${states.join(", ")}`);
    };

    return {
        store,
        clock,
        spaceId,
        served,
        start: (call) =>
            store
                .mutate(
                    store.serving(workflow.run),
                    { id: RequestId.create(clock.now()), calls: [call(spaceId)] },
                    { scope: spaceId },
                )
                .then(([started]) => started),
        asAlice: (calls) =>
            store.mutate(subjectContext(alice, spaceId).access(), {
                id: RequestId.create(clock.now()),
                calls,
            }),
        asBob: (calls) =>
            store.mutate(subjectContext(bob, spaceId).access(), {
                id: RequestId.create(clock.now()),
                calls,
            }),
        runs,
        until,
    };
}

/** Open a new space below a new account, in which anyone may write notes. */
async function openSpace(database: DatabaseConnection): Promise<Identifier<"space">> {
    // replicate the account and the space from their home's access, and create the space's row
    const accountId = schema.identifier("account").parse(`account-${v7()}`);
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    const reference = space.reference(accountId, spaceId);
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(reference);
    await database.insert(space.table).values(
        Table.complete(space.table, {
            id: spaceId,
            scope: accountId,
            name: "notes",
            stack: STACKS.space,
            createdAt: START,
            updatedAt: START,
        }),
    );

    // let anyone read the space and write notes in it, and alice own it
    await access.replicateRole(
        reference,
        {
            name: "member",
            description: "Writes and approves notes",
            permissions: [
                space.permission("read"),
                note.permission("write"),
                note.permission("approve"),
            ],
        },
        anyone.reference("*", "*"),
        START,
    );
    await access.replicateRelationship(spaceId, reference, "owner", alice, START);

    return spaceId;
}

/** The subject a run acts as, alice by default. */
export function actor(subject: Subject = alice): { readonly actor: Subject } {
    return { actor: subject };
}
