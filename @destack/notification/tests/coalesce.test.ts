import { nth } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { document, documentParent } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("replace a keyed notification as unread and collapse a thread's burst into one with a count", async () => {
    await using fixture = await TestNotification.open();
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "editor",
            subject: fixture.subject("bob"),
        }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "editor",
            subject: fixture.subject("carol"),
        }),
    );
    await fixture.call(
        "alice",
        subscription.call("create", { ...fixture.scope, ...documentParent(plan.id) }),
    );

    // publish as bob, and replace the owner's notification by its key once she read it
    await fixture.call(
        "bob",
        document.call("publish", { ...fixture.scope, id: plan.id, state: "publishing" }),
    );
    await fixture.dispatch();
    const status = nth((await fixture.inbox("alice")).entries, 0);
    await fixture.call(
        "alice",
        notification.call("read", {
            ...fixture.homeScope("alice"),
            id: status.notification.id,
        }),
    );
    await fixture.wait(1);
    await fixture.call(
        "bob",
        document.call("publish", { ...fixture.scope, id: plan.id, state: "published" }),
    );
    await fixture.dispatch();

    // show it unread again, its read marker older than the new occurrence, emailing nothing by preference
    const replaced = nth((await fixture.inbox("alice")).entries, 0);
    expect([
        replaced.notification.id === status.notification.id,
        [
            replaced.activity.payload,
            replaced.activity.count,
            replaced.isUnread,
            replaced.activity.occurredAt === fixture.clock.now(),
        ],
        (await fixture.messages("alice", replaced.notification.id)).map((row) => [
            row.to.channel,
            row.status,
            row.reason,
        ]),
    ]).toEqual([true, [{ state: "published" }, 1, true, true], []]);

    // collapse three changes within the burst window into one by its last editor, and start another after it
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "First" }),
    );
    await fixture.wait(5);
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Second" }),
    );
    await fixture.wait(5);
    await fixture.call(
        "carol",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Third" }),
    );
    await fixture.wait(6);
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Fourth" }),
    );
    await fixture.dispatch();
    expect([
        (await fixture.inbox("alice")).unread(),
        (await fixture.activities()).map((row) => [
            row.name,
            row.payload,
            row.count,
            row.reason,
            row.actorId,
        ]),
    ]).toEqual([
        3,
        [
            ["status", { state: "published" }, 1, "author", fixture.subject("bob").id],
            ["change", { summary: "Third" }, 3, "subscribed", fixture.subject("carol").id],
            ["change", { summary: "Fourth" }, 1, "subscribed", fixture.subject("bob").id],
        ],
    ]);
});
