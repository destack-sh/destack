import { TEST_DIALECTS } from "@destack/db/test";
import { aligned } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "replace a keyed notification as unread and collapse a thread's burst into one with a count, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);
        await fixture.reach("alice", { email: "alice@example.com" });
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "editor",
            subject: actors.bob,
        });
        await fixture.call(subscription, "create", documentParent(plan.id));
        const box = fixture.inbox("alice");

        // publish as bob and replace the owner's notification by its key
        fixture.as("bob");
        await fixture.call(document, "publish", { id: plan.id, state: "publishing" });
        await fixture.dispatch();
        const status = aligned(await fixture.notifications(), 0);
        fixture.as("alice");
        await fixture.call(notification, "read", { id: status.id });
        fixture.as("bob");
        fixture.wait(1);
        await fixture.call(document, "publish", { id: plan.id, state: "published" });
        await box.until((state) => state.unread === 1);
        await fixture.dispatch();
        const replaced = aligned(await fixture.notifications(), 0);
        expect([
            replaced.id === status.id,
            [
                aligned(await fixture.activities(), 0).payload,
                replaced.count,
                replaced.readAt,
                replaced.occurredAt === fixture.now(),
            ],
            (await fixture.deliveries(replaced.id)).map((row) => [
                row.channel,
                row.state,
                row.reason,
                row.dueAt === fixture.now(),
            ]),
        ]).toEqual([
            true,
            [{ state: "published" }, 1, null, true],
            [["email", "skipped", "preference", true]],
        ]);

        // collapse three changes within the burst window into one notification, and start another after it
        await fixture.call(document, "edit", { id: plan.id, summary: "First" });
        fixture.wait(5);
        await fixture.call(document, "edit", { id: plan.id, summary: "Second" });
        fixture.wait(5);
        await fixture.call(document, "edit", { id: plan.id, summary: "Third" });
        fixture.wait(6);
        await fixture.call(document, "edit", { id: plan.id, summary: "Fourth" });
        await box.until((state) => state.unread === 3);
        expect(
            (await fixture.activities()).map((row) => [
                row.name,
                row.payload,
                row.count,
                row.reason,
            ]),
        ).toEqual([
            ["status", { state: "published" }, 1, "author"],
            ["change", { summary: "Third" }, 3, "subscribed"],
            ["change", { summary: "Fourth" }, 1, "subscribed"],
        ]);
    },
);
