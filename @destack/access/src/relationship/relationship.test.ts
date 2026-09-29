import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db/log";
import { asc, eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    ACCESS_MAPPINGS,
    accessProposal,
    accessRelationship,
    Authorization,
    Authorizer,
    principal,
    proposal,
    Relationship,
    relationship,
    subjectKey,
    type AccessContext,
} from "../index.ts";
import { mappings, node, policies } from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";

test.for(TEST_DIALECTS)(
    "show a relationship to its subject and to whoever may grant on its object on %s",
    async (dialect) => {
        // open the fixture, where alice owns node b and granted bob editing it
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const authorizer = new Authorizer(policies, [...mappings, ...ACCESS_MAPPINGS]);
        const carol: AccessContext = {
            ...fixture.alice,
            subjects: [principal.user.reference("global", "carol")],
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
    },
);

test.for(TEST_DIALECTS)(
    "show a proposal to its proposer, its addressee and whoever may grant on its object on %s",
    async (dialect) => {
        // open the fixture, where alice owns node b and granted bob editing it
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const authorizer = new Authorizer(policies, [...mappings, ...ACCESS_MAPPINGS]);
        const as = (id: string): AccessContext => ({
            ...fixture.alice,
            subjects: [principal.user.reference("global", id)],
        });

        // let carol ask to view node b, and alice offer dave viewing it
        const object = node.reference("personal", "b");
        await new Authorization(authorizer, fixture.database, () => as("carol")).propose({
            relationship: { object, relation: "viewer", subject: as("carol").subjects[0]! },
        });
        await new Authorization(authorizer, fixture.database, () => fixture.alice).propose({
            relationship: { object, relation: "viewer", subject: as("dave").subjects[0]! },
        });

        // list the proposals each caller may read, by proposer and addressee
        const visible = async (context: AccessContext) => {
            const access = await authorizer.resolve(
                Snapshot.live(fixture.database),
                "personal",
                context,
            );
            const rows = await fixture.database
                .select({
                    proposer: accessProposal.proposerKey,
                    addressee: accessProposal.addressee,
                })
                .from(accessProposal)
                .where(authorizer.where(proposal.permission("read"), access, accessProposal))
                .orderBy(asc(accessProposal.id));

            return rows.map((row) => `${row.proposer}>${row.addressee}`);
        };

        // show both to alice as grantor, and each other caller only its own
        const [carol, dave, alice] = ["carol", "dave", "alice"].map((id) =>
            subjectKey(principal.user.reference("global", id)),
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
        const [first, second, third] = ["a", "b", "c"].map((id) => node.reference("personal", id));
        const held = async () =>
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

        // relate the worker to a and b, keeping bob's grant
        await Relationship.replace(fixture.database, selection, [first!, second!], 1000);
        expect(await held()).toEqual(["a:worker", "b:bob", "b:worker"]);

        // move the worker from a to c, keeping b's relationship and bob's grant
        const [kept] = await fixture.database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(eq(accessRelationship.objectId, "b"))
            .orderBy(asc(accessRelationship.subjectId))
            .limit(1)
            .offset(1);
        await Relationship.replace(fixture.database, selection, [second!, third!], 2000);
        const [unchanged] = await fixture.database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(eq(accessRelationship.id, kept!.id));
        expect([await held(), unchanged]).toEqual([["b:bob", "b:worker", "c:worker"], kept]);

        // remove every relationship of the worker
        await Relationship.replace(fixture.database, selection, [], 3000);
        expect(await held()).toEqual(["b:bob"]);
    },
);
