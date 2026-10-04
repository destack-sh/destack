import { expect, onTestFinished, test } from "@destack/test";
import { Subject } from "@destack/sync";
import { Snapshot, asc, eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    ACCESS_MAPPINGS,
    accessInvitation,
    accessRelationship,
    Authorization,
    Authorizer,
    principal,
    invitation,
    Relationship,
    relationship,
    type AccessContext,
} from "../index.ts";
import { mappings, node, policies } from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";

test.for(TEST_DIALECTS)(
    "show an object's relationships to its readers, a concealed relation's only to its subject and whoever may grant it, on %s",
    async (dialect) => {
        // open the fixture, where alice owns node b and granted bob editing it
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const authorizer = new Authorizer(policies, [...mappings, ...ACCESS_MAPPINGS]);
        const carol: AccessContext = {
            ...fixture.alice,
            subjects: [principal.user.reference("universe", "carol")],
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
                .where(
                    authorizer.where(relationship.permission("read"), access, accessRelationship),
                )
                .orderBy(asc(accessRelationship.id));

            return rows.map((row) => `${row.objectId}:${row.subjectId}`);
        };

        // show the grant to its grantor and its subject, and to no one else
        expect(await visible(fixture.alice)).toEqual(["b:bob"]);
        expect(await visible(fixture.bob)).toEqual(["b:bob"]);
        expect(await visible(carol)).toEqual([]);

        // let carol read node b, and alice add dave as its concealed auditor
        const sharing = new Authorization(authorizer, fixture.database, () => fixture.alice);
        const object = node.reference("personal", "b");
        await sharing.grant({
            object,
            relation: "viewer",
            subject: principal.user.reference("universe", "carol"),
        });
        await sharing.grant({
            object,
            relation: "auditor",
            subject: principal.user.reference("universe", "dave"),
        });
        const dave: AccessContext = {
            ...fixture.alice,
            subjects: [principal.user.reference("universe", "dave")],
        };

        // show every relationship to the reader, the auditor to its granter and itself only
        expect(await visible(carol)).toEqual(["b:bob", "b:carol"]);
        expect(await visible(fixture.bob)).toEqual(["b:bob", "b:carol"]);
        expect(await visible(fixture.alice)).toEqual(["b:bob", "b:carol", "b:dave"]);
        expect(await visible(dave)).toEqual(["b:bob", "b:carol", "b:dave"]);
    },
);

test.for(TEST_DIALECTS)(
    "show an invitation to its inviting principal, its addressee and whoever may grant on its object on %s",
    async (dialect) => {
        // open the fixture, where alice owns node b and granted bob editing it
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const authorizer = new Authorizer(policies, [...mappings, ...ACCESS_MAPPINGS]);
        const as = (id: string): AccessContext => ({
            ...fixture.alice,
            subjects: [principal.user.reference("universe", id)],
        });

        // let carol ask to view node b, and alice offer dave viewing it
        const object = node.reference("personal", "b");
        await new Authorization(authorizer, fixture.database, () => as("carol")).invite({
            relationship: {
                object,
                relation: "viewer",
                subject: principal.user.reference("universe", "carol"),
            },
        });
        await new Authorization(authorizer, fixture.database, () => fixture.alice).invite({
            relationship: {
                object,
                relation: "viewer",
                subject: principal.user.reference("universe", "dave"),
            },
        });

        // list the invitations each caller may read, by inviting principal and addressee
        const visible = async (context: AccessContext) => {
            const access = await authorizer.resolve(
                Snapshot.live(fixture.database),
                "personal",
                context,
            );
            const rows = await fixture.database
                .select({
                    inviter: accessInvitation.inviterKey,
                    addressee: accessInvitation.addressee,
                })
                .from(accessInvitation)
                .where(authorizer.where(invitation.permission("read"), access, accessInvitation))
                .orderBy(asc(accessInvitation.id));

            return rows.map((row) => `${row.inviter}>${row.addressee}`);
        };

        // show both to alice as grantor, and each other caller only its own
        const [carol, dave, alice] = ["carol", "dave", "alice"].map((id) =>
            Subject.key(principal.user.reference("universe", id)),
        );
        expect(await visible(fixture.alice)).toEqual([`${carol}>${carol}`, `${alice}>${dave}`]);
        expect(await visible(as("carol"))).toEqual([`${carol}>${carol}`]);
        expect(await visible(as("dave"))).toEqual([`${alice}>${dave}`]);
        expect(await visible(fixture.bob)).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "replace a subject's relationships through a relation with exactly the wanted objects on %s",
    async (dialect) => {
        // open the fixture, where bob edits node b
        const fixture = await openFixture(dialect);
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
        await Relationship.replace(fixture.database, selection, [first, second], 1000);
        expect(await related()).toEqual(["a:worker", "b:bob", "b:worker"]);

        // move the worker from a to c without touching b's relationship or bob's grant
        const [kept] = await fixture.database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(eq(accessRelationship.objectId, "b"))
            .orderBy(asc(accessRelationship.subjectId))
            .limit(1)
            .offset(1);
        if (kept === undefined) {
            throw new Error("node b keeps no worker relationship");
        }
        await Relationship.replace(fixture.database, selection, [second, third], 2000);
        const [unchanged] = await fixture.database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(eq(accessRelationship.id, kept.id));
        expect([await related(), unchanged]).toEqual([["b:bob", "b:worker", "c:worker"], kept]);

        // remove every relationship of the worker
        await Relationship.replace(fixture.database, selection, [], 3000);
        expect(await related()).toEqual(["b:bob"]);
    },
);
