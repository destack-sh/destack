import type { Environment, HostCapability } from "@destack/environment";
import {
    Grant,
    type GrantPermission,
    InstallationId,
    principal,
    relation,
    universe,
} from "@destack/access";
import { space } from "@destack/account/object";
import type { Simulation } from "@destack/environment/simulation";
import {
    defineObject,
    field,
    Follow,
    grantShape,
    PackageHandle,
    type ObjectType,
} from "@destack/object";
import { DynamicObjectClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import { implementPackage } from "@destack/object/server";
import { TestObjectStore, testSimulation } from "@destack/object/test";
import { Identifier, schema } from "@destack/schema";
import { type Call, RequestId, Scope, type Subject, Uplink } from "@destack/sync";
import { Snapshot } from "@destack/db";
import { onTestFinished } from "@destack/test";
import type {
    RunContext,
    Step,
    WaitOf,
    Workflow,
    WorkflowDeclaration,
    WorkflowImplementation,
    WorkflowServing,
} from "../src/index.ts";
import { approval, changeRequest } from "../src/approval/index.ts";
import workflowPackage from "../src/package.ts";
import workflowServer from "../src/server.ts";
import { NOTES } from "./package.ts";

/** The person starting the runs, who owns the space. */
export const alice = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000100"),
);

/** A person who started no run. */
export const bob = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000101"),
);

/** An agent installed in the space, which people let act for them. */
export function agentOf(spaceId: string): Subject {
    return principal.installation.reference(spaceId, "installation-agent");
}

/** The node the space's store runs as in the fixture's world. */
const SPACE_NODE = "space";

/** The longest a grant takes to reach the space's replica, in milliseconds. */
const REPLICA_MILLISECONDS = 2000;

/** The longest a fixture runs its world toward a condition: twice the longest scenario's three hours. */
const UNTIL_MILLISECONDS = 6 * 60 * 60 * 1000;

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
    NOTES,
);

/** Notes as the fixture serves them, publishing under the given title. */
const servedNote = note.implement({
    methods: { publish: (call) => call.update({ title: call.input.title }) },
});

/** The notes application's package, which clients call for notes. */
export const notesPackage = PackageHandle.of(note.package, { objects: { note } });

/** A space serving notes, the workflow package and a workflow, on a simulated clock. */
export interface WorkflowFixture {
    /** The store serving the space, called as its users. */
    readonly served: TestObjectStore;
    /** The simulated world the store and its controllers run in. */
    readonly simulation: Simulation;
    /** The world the space's store runs in. */
    readonly environment: Environment;
    /** The workflow as the store serves it, its webhooks among it. */
    readonly implementation: WorkflowImplementation;
    /** Call the space's objects by type name as a subject, such as a workflow's runs. */
    readonly objects: (subject: Subject) => DynamicObjectClient;
    /** Let a delegate act for a person, as the person grants at the universe. */
    readonly grant: (
        delegator: Subject,
        delegate: Subject,
        permissions: readonly GrantPermission[],
    ) => Promise<Identifier<"grant">>;
    /** Start a run as the space, acting as the subject the call names. */
    readonly start: (call: (spaceId: string) => Call) => Promise<unknown>;
    /** Read the workflow's runs, oldest first. */
    readonly runs: () => Promise<readonly Record<string, unknown>[]>;
    /** Run the world until a condition holds, failing on the controllers' errors. */
    readonly when: (isMet: () => Promise<boolean>) => Promise<void>;
    /** Run the world until the runs meet a condition, answering them. */
    readonly until: (
        condition: (runs: readonly Record<string, unknown>[]) => boolean,
    ) => Promise<readonly Record<string, unknown>[]>;
}

/** Serve notes and a workflow in alice's space, advancing runs on a simulated clock. */
export async function openWorkflow<Input, Output, Waits extends ObjectType>(
    workflow: Workflow<Input, Output, Waits>,
    code: (step: Step<WaitOf<Waits>>, input: Input, context: RunContext) => Promise<Output>,
    serving: (space: Environment<HostCapability>) => Promise<WorkflowServing> = async () => ({}),
    objects: readonly ObjectType[] = [],
): Promise<WorkflowFixture> {
    // serve the workflow in the space's world
    const simulation = testSimulation();
    const served = await serving(simulation.environment(SPACE_NODE));

    return openServed(workflow, workflow.implement(code, served), [], objects, simulation);
}

/** A further workflow a fixture serves, which the main workflow's runs invoke. */
export interface InvokedServing {
    /** The workflow. */
    readonly workflow: Pick<Workflow, "name" | "objects">;
    /** The workflow as the store serves it. */
    readonly served: WorkflowImplementation;
}

/** Serve notes and served workflows in a space alice owns, advancing runs on a simulated clock. */
export async function openServed<Input, Output>(
    workflow: WorkflowDeclaration<Input, Output>,
    implementation: WorkflowImplementation,
    invoked: readonly InvokedServing[] = [],
    objects: readonly ObjectType[] = [],
    simulation: Simulation = testSimulation(),
): Promise<WorkflowFixture> {
    // declare the notes, the further objects and the workflows as the notes application
    const workflows = [workflow, ...invoked.map((each) => each.workflow)];
    const declared = [
        ...Object.values(notesPackage.objects),
        ...objects,
        ...workflows.flatMap((each) => Object.values(each.objects)),
    ];
    const handle = PackageHandle.of(note.package, {
        objects: Object.fromEntries(declared.map((type) => [type.name, type])),
    });

    // serve the universe where people grant, in the simulated world
    const { clock } = simulation;
    const universal = await TestObjectStore.open({
        packages: [objectPackage],
        scope: universe.reference(Scope.universe.id, Scope.universe.id),
        scopes: [alice, bob],
        simulation,
        node: "universe",
    });
    onTestFinished(() => universal[Symbol.asyncDispose]());

    // follow the grants people give the space and its agent from the universe, as the space
    const spaceId = Identifier.create("space", universal.environment);
    const scope = principal.space.reference(Scope.universe.id, spaceId);
    const runners = await Promise.all(
        [note.package.id, workflowPackage.id].map(async (packageId) =>
            principal.installation.reference(
                spaceId,
                await InstallationId.base(scope, packageId, universal.environment.crypto),
            ),
        ),
    );
    const grants = Follow.of(grantShape, {
        keys: [scope, ...runners, agentOf(spaceId)].map((delegate) => ({
            key: {
                scope: Scope.universe.id,
                delegatePackageId: delegate.packageId,
                delegateType: delegate.type,
                delegateScope: delegate.scope,
                delegateId: delegate.id,
            },
        })),
    });
    const publisher = Uplink.of(() => universal.as(scope).client(objectPackage).store);

    // serve the space's packages beside its access objects in the space alice owns, running their controllers
    const implementations = [implementation, ...invoked.map((each) => each.served)];
    const served = await TestObjectStore.open({
        scope,
        publisher: () => publisher,
        packages: [
            implementPackage(objectPackage, { follows: [grants] }),
            workflowServer,
            implementPackage(handle, {
                objects: [
                    servedNote,
                    ...objects,
                    ...implementations.flatMap((each) => each.objects),
                ],
                controllers: implementations.flatMap((each) => each.controllers),
            }),
        ],
        simulation,
        node: SPACE_NODE,
        owners: [alice],
    });
    onTestFinished(() => served[Symbol.asyncDispose]());

    // let alice and bob read the space and write and approve notes in it, binding the role as alice
    const access = served.as(alice).client(objectPackage);
    const member = await access.role.create({
        scope: served.scope,
        requestId: RequestId.create(served.environment),
        name: "member",
        description: "Writes and approves notes",
        permissions: [
            space.permission("read"),
            note.permission("write"),
            note.permission("approve"),
        ],
    });
    for (const user of [alice, bob]) {
        await access.relationship.create({
            scope: served.scope,
            requestId: RequestId.create(served.environment),
            object: scope,
            role: member.id,
            subject: user,
        });
    }

    // grant at the universe as the delegator, and wait until the space follows the grant
    const grant = async (
        delegator: Subject,
        delegate: Subject,
        permissions: readonly GrantPermission[],
    ) => {
        const created = await universal
            .as(delegator)
            .client(objectPackage)
            .grant.create({
                scope: delegator.id,
                requestId: RequestId.create(served.environment),
                delegate,
                permissions: [...permissions],
            });
        const signal = clock.timeout(REPLICA_MILLISECONDS);
        const isFollowed = async () =>
            (await Grant.read(Snapshot.live(served.database), created.id)) !== undefined;
        if (!(await served.database.log.until(isFollowed, signal))) {
            throw new Error(`the space did not follow grant ${created.id}`);
        }

        return created.id;
    };

    // let alice and bob grant the installations serving the runs every permission of the served types
    const consented = [...declared, ...Object.values(workflowPackage.objects)].flatMap((type) =>
        Object.keys(type.policy.definition.permissions).map((name) => ({
            ...type.policy.permission(name),
            scope: served.scope,
        })),
    );
    for (const user of [alice, bob]) {
        for (const runner of runners) {
            await grant(user, runner, consented);
        }
    }

    // read the runs, oldest first
    const runs = async () =>
        (await served.database.select().from(workflow.run.table)).toSorted((left, right) =>
            schema.string().parse(left["id"]).localeCompare(schema.string().parse(right["id"])),
        );

    // run the world until the condition holds, failing on the controllers' errors first
    const when = async (isMet: () => Promise<boolean>) => {
        try {
            await simulation.run({ until: isMet, to: clock.now() + UNTIL_MILLISECONDS });
        } catch (error) {
            await served.settle();
            throw error;
        }
        await served.settle();
    };

    // run the world until the runs meet the condition, failing with their states
    const until = async (condition: (rows: readonly Record<string, unknown>[]) => boolean) => {
        let rows: readonly Record<string, unknown>[] = [];
        try {
            await when(async () => condition((rows = await runs())));
        } catch (error) {
            const states = rows.map(
                (row) =>
                    `${JSON.stringify(row["state"])}@${JSON.stringify(row["wakeAt"])} ${JSON.stringify(row["failure"])}`,
            );
            throw new Error(`the runs did not reach the condition: ${states.join(", ")}`, {
                cause: error,
            });
        }

        return rows;
    };

    // start a run as the space, which serves the workflow
    const start = async (call: (spaceId: string) => Call) => {
        const [started] = await served.store.mutate(
            served.store.as(workflow.run),
            { id: RequestId.create(served.environment), calls: [call(served.scope)] },
            { scope: served.scope },
        );

        return started;
    };

    // call the served types by name as a subject
    const objectsOf = (subject: Subject) =>
        new DynamicObjectClient({
            objects: declared,
            scope: served.scope,
            endpoint: served.as(subject).endpoint(handle),
            environment: served.environment,
        });

    return {
        served,
        simulation,
        environment: served.environment,
        implementation,
        objects: objectsOf,
        grant,
        start,
        runs,
        when,
        until,
    };
}

/** The subject a run acts as, alice by default. */
export function actor(subject: Subject = alice): { readonly actor: Subject } {
    return { actor: subject };
}
