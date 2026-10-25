import { expect, onTestFinished, refusal, test } from "@destack/test";
import { type ObjectReference, Scope, Subject } from "@destack/sync";
import { nth, present } from "@destack/schema";
import { Snapshot, asc, eq } from "@destack/db";
import {
    ACCESS_MAPPINGS,
    accessInvitation,
    accessRelationship,
    Authorization,
    Authorizer,
    principal,
    invitation,
    Invitation,
    Manager,
    relationship,
    type AccessContext,
    type RelationshipRequest,
    Relationship,
} from "../index.ts";
import { account, accountTable, homeMappings, mappings, node, policies } from "../test/fixture.ts";
import { openFixture, sessionCredential } from "../test/database.ts";

test("show an object's relationships to its readers, a concealed relation's only to its subject and whoever may grant it", async () => {
    // open the fixture, where alice is the owner of node b and granted bob editing it
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const authorizer = new Authorizer(policies, [...mappings, ...ACCESS_MAPPINGS]);
    const carol: AccessContext = {
        ...fixture.alice,
        subject: principal.user.reference("universe", "carol"),
        groups: [],
    };

    // list the relationships each caller may read
    const visible = async (context: AccessContext) => {
        const access = await authorizer.resolve(
            Snapshot.live(fixture.database),
            "personal",
            context,
        );
        const rows = await fixture.database
            .select({
                objectId: accessRelationship.objectId,
                subjectId: accessRelationship.subjectId,
            })
            .from(accessRelationship)
            .where(authorizer.where(relationship.permission("read"), access, accessRelationship))
            .orderBy(asc(accessRelationship.subjectId));

        return rows.map((row) => `${row.objectId}:${row.subjectId}`);
    };

    // show the grant to its grantor and its subject, and to no one else
    expect(await visible(fixture.alice)).toEqual(["b:bob"]);
    expect(await visible(fixture.bob)).toEqual(["b:bob"]);
    expect(await visible(carol)).toEqual([]);

    // let carol read node b, and alice add dave as its concealed auditor
    const sharing = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            authorizer,
            fixture.database,
            () => fixture.alice,
            fixture.environment,
            call,
        );
    const object = node.reference("personal", "b");
    await sharing((authorization) =>
        Relationship.grant(
            {
                object,
                relation: "viewer",
                subject: principal.user.reference("universe", "carol"),
            },
            authorization,
        ),
    );
    await sharing((authorization) =>
        Relationship.grant(
            {
                object,
                relation: "auditor",
                subject: principal.user.reference("universe", "dave"),
            },
            authorization,
        ),
    );
    const dave: AccessContext = {
        ...fixture.alice,
        subject: principal.user.reference("universe", "dave"),
        groups: [],
    };

    // show every relationship to the reader, the auditor to its granter and itself only
    expect(await visible(carol)).toEqual(["b:bob", "b:carol"]);
    expect(await visible(fixture.bob)).toEqual(["b:bob", "b:carol"]);
    expect(await visible(fixture.alice)).toEqual(["b:bob", "b:carol", "b:dave"]);
    expect(await visible(dave)).toEqual(["b:bob", "b:carol", "b:dave"]);
});

test("show an invitation to its inviting principal, its addressee and whoever may grant on its object", async () => {
    // open the fixture, where alice is the owner of node b and granted bob editing it
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const authorizer = new Authorizer(policies, [...mappings, ...ACCESS_MAPPINGS]);
    const as = (id: string): AccessContext => ({
        ...fixture.alice,
        subject: principal.user.reference("universe", id),
        groups: [],
    });

    // let carol ask to view node b, and alice offer dave viewing it
    const object = node.reference("personal", "b");
    await Authorization.run(
        authorizer,
        fixture.database,
        () => as("carol"),
        fixture.environment,
        (authorization) =>
            Invitation.create(
                {
                    relationship: {
                        object,
                        relation: "viewer",
                        subject: principal.user.reference("universe", "carol"),
                    },
                },
                authorization,
            ),
    );
    await Authorization.run(
        authorizer,
        fixture.database,
        () => fixture.alice,
        fixture.environment,
        (authorization) =>
            Invitation.create(
                {
                    relationship: {
                        object,
                        relation: "viewer",
                        subject: principal.user.reference("universe", "dave"),
                    },
                },
                authorization,
            ),
    );

    // list the invitations each caller may read, by inviting principal and addressee
    const visible = async (context: AccessContext) => {
        const access = await authorizer.resolve(
            Snapshot.live(fixture.database),
            "personal",
            context,
        );
        const rows = await fixture.database
            .select({
                inviter: accessInvitation.inviterId,
                addressee: accessInvitation.subjectId,
            })
            .from(accessInvitation)
            .where(authorizer.where(invitation.permission("read"), access, accessInvitation))
            .orderBy(asc(accessInvitation.subjectId));

        return rows.map((row) => `${row.inviter}>${row.addressee}`);
    };

    // show both to alice as grantor, and each other caller only its relationships
    const [carol, dave, alice] = ["carol", "dave", "alice"];
    expect(await visible(fixture.alice)).toEqual([`${carol}>${carol}`, `${alice}>${dave}`]);
    expect(await visible(as("carol"))).toEqual([`${carol}>${carol}`]);
    expect(await visible(as("dave"))).toEqual([`${alice}>${dave}`]);
    expect(await visible(fixture.bob)).toEqual([]);
});

test("read the invitations on an object to one addressee, leaving out other addressees and objects", async () => {
    // let alice offer dave viewing node b
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const object = node.reference("personal", "b");
    const dave = principal.user.reference("universe", "dave");
    await Authorization.run(
        fixture.authorizer,
        fixture.database,
        () => fixture.alice,
        fixture.environment,
        (authorization) =>
            Invitation.create(
                {
                    relationship: { object, relation: "viewer", subject: dave },
                },
                authorization,
            ),
    );

    // read only the offer to dave, none to bob and none to dave on node a
    const snapshot = Snapshot.live(fixture.database);
    const read = async (on: ObjectReference, addressee: Subject) =>
        (await Invitation.readOn(snapshot, on, addressee)).map((offer) => [
            offer.status,
            offer.relationship.object.id,
            offer.relationship.subject.id,
            offer.inviter?.id,
        ]);
    expect([
        await read(object, dave),
        await read(object, principal.user.reference("universe", "bob")),
        await read(node.reference("personal", "a"), dave),
    ]).toEqual([[["pending", "b", "dave", "alice"]], [], []]);
});

test("relate a subject once however often touched, set a new expiry on the same relationship, and refuse a caller who may not grant it", async () => {
    // open the fixture, where alice is the owner of node b and granted bob editing it
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const authorizer = new Authorizer(policies, mappings);
    const shared = node.reference("personal", "b");
    const touch = (context: AccessContext, request: RelationshipRequest) =>
        Authorization.run(
            authorizer,
            fixture.database,
            () => context,
            fixture.environment,
            (authorization) => Relationship.touch(request, authorization),
        );
    const viewer = (id: string): RelationshipRequest => ({
        object: shared,
        relation: "viewer",
        subject: principal.user.reference("universe", id),
    });

    // touch bob's grant and carol's new view twice as alice, and dave's view as bob
    const editing = await touch(fixture.alice, {
        object: shared,
        relation: "editor",
        subject: fixture.bob.subject,
    });
    const first = await touch(fixture.alice, viewer("carol"));
    const again = await touch(fixture.alice, viewer("carol"));
    const lapsing = await touch(fixture.alice, { ...viewer("carol"), expiresAt: 5000 });
    const refused = await refusal(touch(fixture.bob, viewer("dave")));

    // keep one row per relationship, answering the same one each time and setting a new expiry on it
    const rows = await fixture.database
        .select({
            id: accessRelationship.id,
            subjectId: accessRelationship.subjectId,
            expiresAt: accessRelationship.expiresAt,
        })
        .from(accessRelationship)
        .where(eq(accessRelationship.objectId, "b"))
        .orderBy(asc(accessRelationship.subjectId));
    expect({ refused, again: again.id, lapsing: lapsing.id, rows }).toEqual({
        refused: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"],
        again: first.id,
        lapsing: first.id,
        rows: [
            { id: editing.id, subjectId: "bob", expiresAt: null },
            { id: first.id, subjectId: "carol", expiresAt: 5000 },
        ],
    });
});

test("keep an object's last relationship of a required relation through revoking and replacing, and refuse letting one lapse", async () => {
    // create account-1 with alice as its root, and make bob a root too
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const authorizer = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const object = account.reference("universe", "account-1");
    const stepped = {
        ...fixture.alice,
        credential: {
            ...sessionCredential("alice", fixture.alice.now),
            assurance: { level: 2, authenticatedAt: fixture.alice.now },
        },
    };
    const asAlice = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(authorizer, fixture.database, () => stepped, fixture.environment, call);
    const root = (subject: Subject): RelationshipRequest => ({ object, relation: "root", subject });
    await fixture.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    const alice = await asAlice(async (authorization) => {
        await authorization.create(object, {
            relationships: [{ relation: "root", subject: fixture.alice.subject }],
        });
        const [created] = await Relationship.readOn(authorization.transaction.view(), object, {
            relation: "root",
        });

        return present(created, "alice's root relationship");
    });
    const bob = await asAlice((authorization) =>
        Relationship.grant(root(fixture.bob.subject), authorization),
    );

    // revoke bob's root, then refuse revoking alice's, the last one
    await asAlice((authorization) => Relationship.revoke(object, bob.id, authorization));
    const refused = await refusal(
        asAlice((authorization) => Relationship.revoke(object, alice.id, authorization)),
    );

    // refuse a root that would lapse, and replacing alice's roots with none
    const lapsing = await refusal(
        asAlice((authorization) =>
            Relationship.grant(
                { ...root(fixture.bob.subject), expiresAt: fixture.alice.now + 60_000 },
                authorization,
            ),
        ),
    );
    const emptied = await refusal(
        asAlice((authorization) =>
            Relationship.replace(
                {
                    scope: object.scope,
                    objects: [account.typeReference],
                    relation: "root",
                    subject: fixture.alice.subject,
                },
                [],
                authorization,
            ),
        ),
    );

    // keep alice as the one root through every refusal
    const roots = await fixture.database
        .select({ subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(eq(accessRelationship.relation, "root"));
    const last = [
        "CONFLICT",
        "LAST_RELATIONSHIP",
        "the last root of account account-1 cannot be removed",
    ];
    expect({ refused, lapsing, emptied, roots }).toEqual({
        refused: last,
        lapsing: [
            "BAD_REQUEST",
            "INVALID_RELATIONSHIP_EXPIRY",
            "a required relationship cannot expire",
        ],
        emptied: last,
        roots: [{ subjectId: "alice" }],
    });
});

test("replace a subject's relationships through a relation with exactly the wanted objects as a grantor, refusing anyone else", async () => {
    // open the fixture, where bob edits node b
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const worker = principal.installation.reference("personal", "worker");
    const selection = {
        scope: "personal",
        objects: [{ packageId: node.definition.packageId, type: node.name }],
        relation: "editor",
        subject: worker,
    };
    const first = node.reference("personal", "a");
    const second = node.reference("personal", "b");
    const third = node.reference("personal", "c");
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            fixture.authorizer,
            fixture.database,
            () => fixture.alice,
            fixture.environment,
            call,
        );
    const wanting = (...objects: ObjectReference[]) =>
        objects.map((object) => ({ object, relation: "editor", subject: worker }));
    const related = async () =>
        (
            await fixture.database
                .select({
                    objectId: accessRelationship.objectId,
                    subjectId: accessRelationship.subjectId,
                })
                .from(accessRelationship)
                .where(eq(accessRelationship.relation, "editor"))
                .orderBy(asc(accessRelationship.objectId), asc(accessRelationship.subjectId))
        ).map((row) => `${row.objectId}:${row.subjectId}`);

    // relate the worker to a and b next to bob's grant
    await owner((authorization) =>
        Relationship.replace(selection, wanting(first, second), authorization),
    );
    expect(await related()).toEqual(["a:worker", "b:bob", "b:worker"]);

    // move the worker from a to c without touching b's relationship or bob's grant
    const [stored] = await fixture.database
        .select({ id: accessRelationship.id })
        .from(accessRelationship)
        .where(eq(accessRelationship.objectId, "b"))
        .orderBy(asc(accessRelationship.subjectId))
        .limit(1)
        .offset(1);
    if (stored === undefined) {
        throw new Error("node b has no worker relationship");
    }
    await owner((authorization) =>
        Relationship.replace(selection, wanting(second, third), authorization),
    );
    const [unchanged] = await fixture.database
        .select({ id: accessRelationship.id })
        .from(accessRelationship)
        .where(eq(accessRelationship.id, stored.id));
    expect([await related(), unchanged]).toEqual([["b:bob", "b:worker", "c:worker"], stored]);

    // remove every relationship of the worker
    await owner((authorization) => Relationship.replace(selection, wanting(), authorization));
    expect(await related()).toEqual(["b:bob"]);

    // refuse bob, who edits b without sharing it
    expect(
        await refusal(
            Authorization.run(
                fixture.authorizer,
                fixture.database,
                () => fixture.bob,
                fixture.environment,
                (authorization) => Relationship.replace(selection, wanting(second), authorization),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"]);
});

test("store a declaration's relationships under stable identifiers, refuse revoking them from anyone else, and leave them once released", async () => {
    // open the fixture, where alice is the owner of every node
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            fixture.authorizer,
            fixture.database,
            () => fixture.alice,
            fixture.environment,
            call,
        );
    const manager = Manager.schema.parse({
        installationId: "installation-01996ab0-0000-7000-8000-000000000001",
        packageId: "package-01996ab0-0000-7000-8000-000000000002",
        name: "editor",
    });
    const declaring = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        owner((authorization) => call(authorization.applying(manager)));
    const selection = { scope: "personal" };
    const managed = async () =>
        (
            await fixture.database
                .select({ id: accessRelationship.id, objectId: accessRelationship.objectId })
                .from(accessRelationship)
                .where(eq(accessRelationship.managerName, "editor"))
        ).map((row) => [row.objectId, row.id] as const);

    // store carol's managed grant on a twice under the same identifier
    await declaring((authorization) =>
        Relationship.replace(selection, [carolEditing("a")], authorization),
    );
    const first = await managed();
    await declaring((authorization) =>
        Relationship.replace(selection, [carolEditing("a")], authorization),
    );
    expect([first.length, nth(first, 0)[0], await managed()]).toEqual([1, "a", first]);

    // refuse alice and another declaration of the package revoking the managed grant, and move it through its declaration
    const id = nth(nth(first, 0), 1);
    const revoke = (authorization: Authorization) =>
        Relationship.revoke(node.reference("personal", "a"), id, authorization);
    const other = { ...manager, name: "viewer" };
    const refused = [
        await refusal(owner(revoke)),
        await refusal(owner((authorization) => revoke(authorization.applying(other)))),
    ];
    await declaring((authorization) =>
        Relationship.replace(selection, [carolEditing("c")], authorization),
    );
    const moved = await managed();

    // release the grant as its stack does, then keep it through the declaration's next replace and let alice revoke it
    const [, released] = present(moved[0], "the moved grant");
    await fixture.database.transaction(
        async (transaction) =>
            await transaction
                .update(accessRelationship)
                .set({ detachedAt: 1 })
                .where(eq(accessRelationship.id, released)),
    );
    await declaring((authorization) => Relationship.replace(selection, [], authorization));
    const kept = await managed();
    const revoked = await refusal(
        owner((authorization) =>
            Relationship.revoke(node.reference("personal", "c"), released, authorization),
        ),
    );
    const message = "relationship is managed by its declaration: detach it before changing it";
    expect({ refused, moved: moved.map(([objectId]) => objectId), kept, revoked }).toEqual({
        refused: [
            ["CONFLICT", "MANAGED", message],
            ["CONFLICT", "MANAGED", message],
        ],
        moved: ["c"],
        kept: moved,
        revoked: "done",
    });
});

/** Request carol's editing of one personal node. */
function carolEditing(id: string): RelationshipRequest {
    return {
        object: node.reference("personal", id),
        relation: "editor",
        subject: principal.user.reference("universe", "carol"),
    };
}
