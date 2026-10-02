import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "replace a keyed notification as unread and collapse a thread's burst into one with a count, on %s",
    async (dialect) => {
        const { call, as, dispatch, inbox, wait, now, notifications, deliveries, host, homes } =
            await serveSpace(dialect);
        homes.emails.set(Subject.key(actors.alice), "alice@example.com");
        const plan = await call("create", { title: "Launch plan" });
        await call("grant", { id: plan.id, relation: "editor", subject: actors.bob });
        await call("create", host(plan.id), subscription);
        const box = inbox("alice");

        // publish as bob and replace the owner's notification by its key
        as("bob");
        await call("publish", { id: plan.id, state: "publishing" });
        await dispatch();
        const [status] = await notifications();
        as("alice");
        await call("read", { id: status!.id }, notification);
        as("bob");
        wait(1);
        await call("publish", { id: plan.id, state: "published" });
        await box.until((state) => state.unread === 1);
        await dispatch();
        const [replaced] = await notifications();
        expect([
            replaced!.id === status!.id,
            [replaced!.payload, replaced!.count, replaced!.readAt, replaced!.occurredAt === now()],
            (await deliveries(replaced!.id)).map((row) => [
                row.channel,
                row.state,
                row.reason,
                row.dueAt === now(),
            ]),
        ]).toEqual([
            true,
            [{ state: "published" }, 1, null, true],
            [["email", "skipped", "preference", true]],
        ]);

        // collapse three changes within the burst window into one notification, and start another after it
        await call("edit", { id: plan.id, summary: "First" });
        wait(5);
        await call("edit", { id: plan.id, summary: "Second" });
        wait(5);
        await call("edit", { id: plan.id, summary: "Third" });
        wait(6);
        await call("edit", { id: plan.id, summary: "Fourth" });
        await box.until((state) => state.unread === 3);
        expect(
            (await notifications()).map((row) => [row.name, row.payload, row.count, row.reason]),
        ).toEqual([
            ["status", { state: "published" }, 1, "author"],
            ["change", { summary: "Third" }, 3, "subscribed"],
            ["change", { summary: "Fourth" }, 1, "subscribed"],
        ]);
    },
);
