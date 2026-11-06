import { eq } from "@destack/db";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { activity, notification } from "../src/object/index.ts";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("plan an activity's notification as the activity changes, again once the notification is stale or missing", async () => {
    // open bob's home, mention him once and plan his notification
    await using fixture = await TestNotification.open();
    const { served: home } = await fixture.home("bob");
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
    await fixture.simulation.idle();
    const [mentioned] = await home.database.select().from(activity.table);
    const { id, occurredAt } = present(mentioned, "bob's activity");
    const table = notification.table;
    const planned = async () =>
        (await home.database.select({ plannedAt: table.plannedAt }).from(table)).map(
            (row) => row.plannedAt,
        );
    const occur = async (at: number) => {
        await home.database.transaction(
            async (transaction) =>
                await transaction
                    .update(activity.table)
                    .set({ occurredAt: at })
                    .where(eq(activity.table.id, id)),
        );
        await fixture.simulation.idle();
    };
    const first = await planned();

    // plan it again once it was planned before a later occurrence, then once it is missing
    await home.database.transaction(
        async (transaction) => await transaction.update(table).set({ plannedAt: occurredAt - 1 }),
    );
    await occur(occurredAt + 1);
    const stale = await planned();
    await home.database.transaction(async (transaction) => await transaction.delete(table));
    await occur(occurredAt + 2);
    const missing = await planned();
    expect({ first, stale, missing }).toEqual({
        first: [occurredAt],
        stale: [occurredAt + 1],
        missing: [occurredAt + 2],
    });
});
