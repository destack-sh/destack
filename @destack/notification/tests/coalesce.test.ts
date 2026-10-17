import { TEST_DIALECTS } from "@destack/db/test";
import { nth } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "replace a keyed notification as unread and collapse a thread's burst into one with a count, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);
        await fixture.contact("alice", { email: "alice@example.com" });
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "editor",
                subject: actors.bob,
            }),
        );
        await fixture.call(
            subscription.call("create", { ...fixture.scope, ...documentParent(plan.id) }),
        );

        // publish as bob, and replace the owner's notification by its key once she read it
        fixture.as("bob");
        await fixture.call(
            document.call("publish", { ...fixture.scope, id: plan.id, state: "publishing" }),
        );
        await fixture.dispatch();
        const status = nth((await fixture.inbox("alice")).entries, 0);
        fixture.as("alice");
        await fixture.call(
            notification.call("read", {
                ...(await fixture.homeScope()),
                id: status.notification.id,
            }),
        );
        fixture.as("bob");
        await fixture.wait(1);
        await fixture.call(
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

        // collapse three changes within the burst window into one notification, and start another after it
        await fixture.call(
            document.call("edit", { ...fixture.scope, id: plan.id, summary: "First" }),
        );
        await fixture.wait(5);
        await fixture.call(
            document.call("edit", { ...fixture.scope, id: plan.id, summary: "Second" }),
        );
        await fixture.wait(5);
        await fixture.call(
            document.call("edit", { ...fixture.scope, id: plan.id, summary: "Third" }),
        );
        await fixture.wait(6);
        await fixture.call(
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
            ]),
        ]).toEqual([
            3,
            [
                ["status", { state: "published" }, 1, "author"],
                ["change", { summary: "Third" }, 3, "subscribed"],
                ["change", { summary: "Fourth" }, 1, "subscribed"],
            ],
        ]);
    },
);
