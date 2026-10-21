import { eq, Snapshot } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { nth, present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { activity, notification } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "list on start only the activities whose notification is missing or planned before their last occurrence, on %s",
    async (dialect) => {
        // open bob's home, mention him once and plan his notification
        const fixture = await serveSpace(dialect);
        const { test: home, store } = await fixture.home("bob");
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
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@bob, look",
                mentions: [actors.bob],
            }),
        );
        await fixture.dispatch();
        const controller = present(
            store.controllers().find((each) => each.name === activity.name),
            "the activity controller",
        );
        const list = async () => controller.list(Snapshot.live(home.database));
        const [mentioned] = await home.database.select().from(activity.table);
        const id = present(mentioned, "bob's activity").id;
        const planned = await list();

        // list it again once its notification was planned before its last occurrence, then once it is missing
        const table = notification.table;
        await home.database
            .update(table)
            .set({ plannedAt: 0 })
            .where(
                eq(table.id, nth(await home.database.select({ id: table.id }).from(table), 0).id),
            );
        const stale = await list();
        await home.database.delete(table);
        const missing = await list();
        expect({ planned, stale, missing }).toEqual({
            planned: [],
            stale: [JSON.stringify({ id })],
            missing: [JSON.stringify({ id })],
        });
    },
);
