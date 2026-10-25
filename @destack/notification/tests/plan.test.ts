import { eq, Snapshot } from "@destack/db";
import { nth, present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { activity, notification } from "../src/index.ts";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("list on start only the activities whose notification is missing or planned before their last occurrence", async () => {
    // open bob's home, mention him once and plan his notification
    await using fixture = await TestNotification.open();
    const { served: home } = await fixture.home("bob");
    const { store } = home;
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
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text: "@bob, look",
            mentions: [fixture.subject("bob")],
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
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .update(table)
                .set({ plannedAt: 0 })
                .where(
                    eq(table.id, nth(await transaction.select({ id: table.id }).from(table), 0).id),
                ),
    );
    const stale = await list();
    await home.database.transaction(async (transaction) => await transaction.delete(table));
    const missing = await list();
    expect({ planned, stale, missing }).toEqual({
        planned: [],
        stale: [JSON.stringify({ id })],
        missing: [JSON.stringify({ id })],
    });
});
