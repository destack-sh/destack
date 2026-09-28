import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db/log";
import { TEST_DIALECTS } from "@destack/db/test";
import { v7 } from "uuid";
import { accessRelationship, principal, Relationship, subjectKey, type Subject } from "../index.ts";
import { group, node } from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";

test.for(TEST_DIALECTS)(
    "list the users holding a permission on an object, through nested groups, deciding each as a check does, on %s",
    async (dialect) => {
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const { database, authorizer } = fixture;
        const user = (id: string) => principal.user.reference("global", id);
        const relate = (object: Relationship["object"], relation: string, subject: Subject) =>
            database.insert(accessRelationship).values(
                Relationship.encode(
                    {
                        id: `relationship-${v7()}`,
                        object,
                        relation,
                        subject,
                        createdAt: 1,
                        expiresAt: null,
                    },
                    object.scope,
                ) as never,
            );

        // let a group, holding carol and a nested group holding dave, and every user view note a
        const outer = group.reference("personal", "outer");
        const inner = group.reference("personal", "inner");
        await relate(node.reference("personal", "a"), "viewer", { ...outer, relation: "member" });
        await relate(node.reference("personal", "a"), "viewer", principal.user.reference("*", "*"));
        await relate(outer, "member", user("carol"));
        await relate(outer, "member", { ...inner, relation: "member" });
        await relate(inner, "member", user("dave"));

        // list alice as owner, carol and dave through the groups, leaving out the wildcard; b's viewers of a do not inherit it
        const readers = async (id: string) =>
            (
                await authorizer.subjects(
                    Snapshot.live(database),
                    node.permission("read"),
                    node.reference("personal", id),
                    {
                        packageId: principal.user.definition.packageId,
                        type: principal.user.definition.name,
                    },
                    1000,
                    { limit: 10 },
                )
            )
                .map((subject) => subject.id)
                .sort();
        // walk note a's readers two at a time in key order, each page after the last one's key
        const page = (after?: string) =>
            authorizer.subjects(
                Snapshot.live(database),
                node.permission("read"),
                node.reference("personal", "a"),
                {
                    packageId: principal.user.definition.packageId,
                    type: principal.user.definition.name,
                },
                1000,
                { ...(after === undefined ? {} : { after }), limit: 2 },
            );
        const first = await page();
        const second = await page(subjectKey(first.at(-1)!));
        expect([first.map((subject) => subject.id), second.map((subject) => subject.id)]).toEqual([
            ["alice", "carol"],
            ["dave"],
        ]);
        expect([await readers("a"), await readers("b")]).toEqual([
            ["alice", "carol", "dave"],
            ["alice", "bob"],
        ]);
    },
);
