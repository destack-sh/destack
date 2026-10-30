import {
    Grant,
    type GrantPermission,
    principal,
    Relationship,
    relation,
    universe,
} from "@destack/access";
import { Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import { Package } from "@destack/package";
import { Identifier, schema } from "@destack/schema";
import { RequestId, Scope, type Subject, Uplink } from "@destack/sync";
import { onTestFinished } from "@destack/test";
import {
    approval,
    type Approval,
    changeRequest,
    defineObject,
    field,
    Follow,
    grantShape,
    PackageHandle,
} from "../../src/index.ts";
import objectPackage from "../../src/package.ts";
import { implementPackage } from "../../src/server/index.ts";
import { TestObjectStore, testSimulation } from "../../src/test/index.ts";

/** The longest a grant takes to reach the space's replica, in milliseconds. */
const REPLICA_MILLISECONDS = 2000;

/** The longest a fixture runs its world toward a condition: an hour, past any change's settling. */
const UNTIL_MILLISECONDS = 60 * 60 * 1000;

/** The module of the notes application whose calls the changes make. */
const NOTES = {
    package: Package.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000f1",
        name: "@example/notes",
        version: "2026.10.0",
    }),
};

/** The person who owns the space. */
export const alice = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000100"),
);

/** A member of alice's space. */
export const bob = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000101"),
);

/** A member of alice's space who approves notes, never asked by default. */
export const carol = principal.user.reference(
    Scope.universe.id,
    schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000102"),
);

/** An agent installed in the space, which people let act for them. */
export function agentOf(spaceId: string): Subject {
    return principal.installation.reference(spaceId, "installation-agent");
}

/** Notes their owners write, and publish under a new title, which no role lets anyone else do. */
export const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: [principal.space],
        fields: { owner: field.subject().caller(), title: field.string() },
        permissions: {
            read: relation("owner"),
            write: relation("owner"),
            approve: relation("owner"),
            publish: relation("owner"),
            share: relation("owner"),
        },
        // let the owner bind the space's roles on a note
        shareable: { by: "share" },
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

/** The notes application's package, which clients call for notes. */
export const notesPackage = PackageHandle.of(note.package, { objects: { note } });

/** A space serving notes and change requests over them, on a simulated clock. */
export interface ApprovalFixture {
    /** The store serving the space, called as its users. */
    readonly served: TestObjectStore;
    /** The world the space's store runs in. */
    readonly environment: Environment;
    /** Let a delegate act for a person, as the person grants at the universe. */
    readonly grant: (
        delegator: Subject,
        delegate: Subject,
        permissions: readonly GrantPermission[],
    ) => Promise<Identifier<"grant">>;
    /** Run the world until a condition holds, failing on the controllers' errors. */
    readonly when: (isMet: () => Promise<boolean>) => Promise<void>;
    /** Write a note as alice, which bob and carol approve as members, answering its identifier. */
    readonly write: () => Promise<string>;
    /** Bind a role of the space on a note to a subject, as alice. */
    readonly bind: (id: string, role: string, subject: Subject) => Promise<void>;
    /** Bind a role across the space to a subject, as the space's declarations do. */
    readonly assign: (role: string, subject: Subject) => Promise<void>;
    /** Decide an approval as its approver. */
    readonly decide: (
        approver: Subject,
        asked: Approval,
        decision: "approve" | "reject",
    ) => Promise<void>;
}

/** Serve notes and change requests in alice's space, where bob and carol approve alice's notes. */
export async function openApprovals(): Promise<ApprovalFixture> {
    // serve the universe where people grant, in the simulated world
    const simulation = testSimulation();
    const { clock } = simulation;
    const universal = await TestObjectStore.open({
        packages: [objectPackage],
        scope: universe.reference(Scope.universe.id, Scope.universe.id),
        scopes: [alice, bob, carol],
        simulation,
        node: "universe",
    });
    onTestFinished(() => universal[Symbol.asyncDispose]());

    // follow the grants people give the space and its agent from the universe, as the space
    const spaceId = Identifier.create("space", universal.environment);
    const scope = principal.space.reference(Scope.universe.id, spaceId);
    const grants = Follow.of(grantShape, {
        keys: [scope, agentOf(spaceId)].map((delegate) => ({
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

    // serve the space's notes beside its access and change requests in the space alice owns
    const served = await TestObjectStore.open({
        scope,
        publisher: () => publisher,
        packages: [
            implementPackage(objectPackage, { follows: [grants] }),
            implementPackage(notesPackage, {
                objects: [
                    note.implement({ methods: { publish: (call) => call.update(call.input) } }),
                ],
            }),
        ],
        simulation,
        node: "space",
        owners: [alice],
    });
    onTestFinished(() => served[Symbol.asyncDispose]());

    // define the members' role approving notes, which alice binds on each note she writes
    const access = served.as(alice).client(objectPackage);
    const member = await access.role.create({
        scope: served.scope,
        requestId: RequestId.create(served.environment),
        name: "member",
        description: "Reads and approves notes",
        permissions: [note.permission("read"), note.permission("approve")],
    });
    const bind = async (id: string, role: string, subject: Subject) => {
        await access.relationship.create({
            scope: served.scope,
            requestId: RequestId.create(served.environment),
            object: note.reference(served.scope, id),
            role,
            subject,
        });
    };
    const assign = async (role: string, subject: Subject) => {
        await Relationship.insert(served.database, {
            object: scope,
            role,
            subject,
            id: Identifier.create("relationship", served.environment),
            createdAt: served.environment.clock.now(),
            expiresAt: null,
        });
    };
    const write = async () => {
        const { id } = await served
            .as(alice)
            .client(notesPackage)
            .note.create({
                scope: served.scope,
                requestId: RequestId.create(served.environment),
                title: "Plan",
            });
        for (const subject of [bob, carol]) {
            await bind(id, member.id, subject);
        }

        return id;
    };

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

    // run the world until the condition holds, failing on the controllers' errors first
    const when = async (isMet: () => Promise<boolean>) => {
        try {
            await simulation.until(isMet, clock.now() + UNTIL_MILLISECONDS);
        } catch (error) {
            await served.settle();
            throw error;
        }
        await served.settle();
    };

    // decide an approval as its approver through the package's client
    const decide = async (approver: Subject, asked: Approval, decision: "approve" | "reject") => {
        const approvals = served.as(approver).client(objectPackage).approval;
        await approvals[decision]({
            ...approval.target(asked),
            requestId: RequestId.create(served.environment),
        });
    };

    return { served, environment: served.environment, grant, when, write, bind, assign, decide };
}
