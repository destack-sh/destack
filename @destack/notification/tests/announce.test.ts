import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Activity, subscription } from "../src/index.ts";
import { document, documentParent } from "../src/test/notes.ts";
import { type Person, TestNotification } from "../src/test/notification.ts";

test("expand an announcement to every subscriber, or every member, in batches, never to its author", async () => {
    await using fixture = await TestNotification.open({ batch: 2 });
    const { scope } = fixture;
    const states = async () =>
        byRecipient(
            (await fixture.activities()).map((row) => [
                Subject.key(Activity.recipient(row)),
                row.reason,
                row.count,
            ]),
        );
    const owner = Subject.key(fixture.space.owner.subject);
    const state = (person: Person, reason: string, count: number) => [
        Subject.key(fixture.subject(person)),
        reason,
        count,
    ];

    // subscribe the author and four viewers to a document
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        subscription.call("create", { ...scope, ...documentParent(plan.id) }),
    );
    for (const viewer of ["bob", "carol", "dave", "erin"] as const) {
        await fixture.call(
            "alice",
            document.call("grant", {
                ...scope,
                id: plan.id,
                relation: "viewer",
                subject: fixture.subject(viewer),
            }),
        );
        await fixture.call(
            viewer,
            subscription.call("create", { ...scope, ...documentParent(plan.id) }),
        );
    }

    // announce the document to its subscribers as one announcement, expanded two at a time
    await fixture.call(
        "alice",
        document.call("broadcast", { ...scope, id: plan.id, audience: "subscribers" }),
    );
    await fixture.settle();
    const subscribed = await states();

    // announce it to the space's members, its account's owner among them, collapsing into their unread notifications
    for (const member of ["alice", "bob", "carol"] as const) {
        await fixture.join(member);
    }
    await fixture.call(
        "alice",
        document.call("broadcast", { ...scope, id: plan.id, audience: "members" }),
    );
    await fixture.settle();
    const members = await states();

    // announce it to the users with edit, found through their grants, in batches of two
    for (const editor of ["dave", "erin"] as const) {
        await fixture.call(
            "alice",
            document.call("grant", {
                ...scope,
                id: plan.id,
                relation: "editor",
                subject: fixture.subject(editor),
            }),
        );
    }
    await fixture.call(
        "alice",
        document.call("broadcast", { ...scope, id: plan.id, audience: "editors" }),
    );
    await fixture.settle();
    const editors = await states();

    expect([subscribed, members, editors]).toEqual([
        byRecipient([
            state("bob", "subscribed", 1),
            state("carol", "subscribed", 1),
            state("dave", "subscribed", 1),
            state("erin", "subscribed", 1),
        ]),
        byRecipient([
            [owner, "mention", 1],
            state("bob", "mention", 2),
            state("carol", "mention", 2),
            state("dave", "subscribed", 1),
            state("erin", "subscribed", 1),
        ]),
        byRecipient([
            [owner, "mention", 2],
            state("bob", "mention", 2),
            state("carol", "mention", 2),
            state("dave", "mention", 2),
            state("erin", "mention", 2),
        ]),
    ]);
});

test("notify each subscriber for the reason it subscribed, skipping the principals an announcement excludes", async () => {
    await using fixture = await TestNotification.open();
    const { scope } = fixture;

    // subscribe bob by a mention, carol and dave by hand
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...scope, title: "Launch plan" }),
    );
    for (const viewer of ["bob", "carol", "dave"] as const) {
        await fixture.call(
            "alice",
            document.call("grant", {
                ...scope,
                id: plan.id,
                relation: "viewer",
                subject: fixture.subject(viewer),
            }),
        );
    }
    await fixture.call(
        "alice",
        document.call("remark", {
            ...scope,
            id: plan.id,
            text: "@bob, a look?",
            mentions: [fixture.subject("bob")],
        }),
    );
    for (const viewer of ["carol", "dave"] as const) {
        await fixture.call(
            viewer,
            subscription.call("create", { ...scope, ...documentParent(plan.id) }),
        );
    }

    // announce to the subscribers but dave, as a call that told dave already
    await fixture.call(
        "alice",
        document.call("broadcast", {
            ...scope,
            id: plan.id,
            audience: "subscribers",
            excluded: [fixture.subject("dave")],
        }),
    );
    await fixture.settle();
    const announced = (await fixture.activities()).filter((row) => row.name === "change");
    expect(
        byRecipient(announced.map((row) => [Subject.key(Activity.recipient(row)), row.reason])),
    ).toEqual(
        byRecipient([
            [Subject.key(fixture.subject("bob")), "mention"],
            [Subject.key(fixture.subject("carol")), "subscribed"],
        ]),
    );
});

/** Order rows by their first column, a recipient's key, as signed-in people's keys vary by run. */
function byRecipient(rows: readonly (readonly unknown[])[]): readonly (readonly unknown[])[] {
    return rows.toSorted((left, right) => String(left[0]).localeCompare(String(right[0])));
}
