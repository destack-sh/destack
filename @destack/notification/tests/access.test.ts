import { nth } from "@destack/schema";
import { expect, test } from "@destack/test";
import { focus } from "../src/index.ts";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("drop an activity from a recipient's home once they lose access to its source, sending nothing, and replicate it back on a regained share", async () => {
    await using fixture = await TestNotification.open();

    // give carol a phone and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
    await fixture.contact("carol", { timeZone: "Europe/Vienna" });
    await fixture.browser("carol", "/carol");
    await fixture.set("carol", focus, {
        schedules: [{ days: [1], from: "11:00", to: "14:00" }],
        allowed: [],
        isTimeSensitiveAllowed: true,
    });

    // mention carol on a document she views and schedule the alerts for the end of her quiet hours at 12:00 UTC
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    const { id: relationship } = await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "viewer",
            subject: fixture.subject("carol"),
        }),
    );
    await fixture.call(
        "alice",
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text: "@carol, a question",
            mentions: [fixture.subject("carol")],
        }),
    );
    await fixture.simulation.idle();
    const mentioned = nth((await fixture.inbox("carol")).entries, 0);
    const end = Date.UTC(2026, 8, 28, 12, 0);
    const read = async () =>
        (await fixture.messages("carol", mentioned.notification.id)).map((row) => [
            row.to.channel,
            row.status,
        ]);
    expect(await read()).toEqual([]);

    // revoke carol's access, which drops the activity from her home and empties her inbox
    await fixture.call(
        "alice",
        document.call("revoke", {
            ...fixture.scope,
            id: plan.id,
            relationshipId: relationship,
        }),
    );
    await fixture.simulation.idle();
    const revoked = (await fixture.inbox("carol")).entries;

    // send nothing once the quiet hours end, the delivery ending without the activity her home no longer has
    await fixture.reach(end);
    expect([
        revoked,
        fixture.clock.now(),
        fixture.pushes.requests.length,
        fixture.mails.sent.length,
        await read(),
    ]).toEqual([[], end, 0, 0, []]);

    // replicate the kept activity back once carol regains her share, unread, alerting nothing again
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "viewer",
            subject: fixture.subject("carol"),
        }),
    );
    await fixture.simulation.idle();
    const regained = (await fixture.inbox("carol")).entries;
    expect([
        regained.map((entry) => [entry.notification.id, entry.isUnread]),
        fixture.pushes.requests.length,
        fixture.mails.sent.map((mail) => mail.to),
    ]).toEqual([[[mentioned.notification.id, true]], 0, []]);
});
