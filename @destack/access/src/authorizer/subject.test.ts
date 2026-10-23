import { expect, onTestFinished, test } from "@destack/test";
import { Subject } from "@destack/sync";
import { Snapshot } from "@destack/db";
import { nth } from "@destack/schema";
import { v7 } from "uuid";
import { accessRelationship, principal, Relationship } from "../index.ts";
import { group, node } from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";

/** Reference a user in the universe. */
function user(id: string): Subject {
    return principal.user.reference("universe", id);
}

test("list the users with a permission on an object, through nested groups, deciding each as a check does", async () => {
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const { database, authorizer } = fixture;
    const relate = (object: Relationship["object"], relation: string, subject: Subject) =>
        database.insert(accessRelationship).values(
            Relationship.encode({
                id: `relationship-${v7()}`,
                object,
                relation,
                subject,
                createdAt: 1,
                expiresAt: null,
            }),
        );

    // let a group, with carol and a nested group with dave, and every user view note a
    const outer = group.reference("personal", "outer");
    const nested = group.reference("personal", "nested");
    await relate(node.reference("personal", "a"), "viewer", { ...outer, relation: "member" });
    await relate(node.reference("personal", "a"), "viewer", principal.user.reference("*", "*"));
    await relate(outer, "member", user("carol"));
    await relate(outer, "member", { ...nested, relation: "member" });
    await relate(nested, "member", user("dave"));

    // list alice as owner and carol and dave through the groups, without the wildcard or b's viewers
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
            .toSorted();
    // walk note a's readers in pages of two in key order
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
    const second = await page(Subject.key(nth(first, -1)));
    expect([first.map((subject) => subject.id), second.map((subject) => subject.id)]).toEqual([
        ["alice", "carol"],
        ["dave"],
    ]);
    expect([await readers("a"), await readers("b")]).toEqual([
        ["alice", "carol", "dave"],
        ["alice", "bob"],
    ]);
});
