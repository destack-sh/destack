import { TEST_DIALECTS } from "@destack/db/test";
import { nth, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Activity, activity, focus } from "../src/index.ts";
import { ObjectReference } from "@destack/sync";
import { subscribeBrowser } from "@destack/message/test";
import { document, mention } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "drop an activity from a recipient's home once they lose access to its source, sending nothing, and replicate it back on a regained share, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give carol a phone, an email address and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
        const phone = await subscribeBrowser();
        await fixture.contact("carol", { email: "carol@example.com", timeZone: "Europe/Vienna" });
        await fixture.subscribe("carol", {
            id: schema.identifier("client").parse("client-019f5530-8000-7000-8000-00000000000c"),
            url: "https://push.example/carol",
            keys: phone.keys,
        });
        await fixture.set("carol", focus, {
            schedules: [{ days: [1], from: "11:00", to: "14:00" }],
            allowed: [],
            isTimeSensitiveAllowed: true,
        });

        // mention carol on a document she views and schedule the alerts for the end of her quiet hours at 12:00 UTC
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        const { id: relationship } = await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "viewer",
                subject: actors.carol,
            }),
        );
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@carol, a question",
                mentions: [actors.carol],
            }),
        );
        await fixture.dispatch();
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
            document.call("revoke", {
                ...fixture.scope,
                id: plan.id,
                relationshipId: relationship,
            }),
        );
        await fixture.dispatch();
        const revoked = (await fixture.inbox("carol")).entries;

        // send nothing once the quiet hours end, the delivery ending without the activity her home no longer has
        await fixture.wait(120);
        await fixture.dispatch();
        expect([
            revoked,
            fixture.clock.now(),
            fixture.pushes.requests.length,
            fixture.mails.sent.length,
            await read(),
        ]).toEqual([[], end, 0, 0, []]);

        // replicate the kept activity back once carol regains her share, unread, alerting nothing again
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "viewer",
                subject: actors.carol,
            }),
        );
        await fixture.dispatch();
        const regained = (await fixture.inbox("carol")).entries;
        expect([
            regained.map((entry) => [entry.notification.id, entry.isUnread]),
            fixture.pushes.requests.length,
            fixture.mails.sent.map((mail) => mail.to),
        ]).toEqual([[[mentioned.notification.id, true]], 0, []]);
    },
);

test("post an activity through notify as its calling reader, and refuse a caller who cannot read its source", async () => {
    // share a document only with bob
    const fixture = await serveSpace("sqlite");
    const plan = await fixture.call(
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    await fixture.call(
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "viewer",
            subject: actors.bob,
        }),
    );
    const notice = {
        ...documentParent(plan.id),
        packageId: mention.reference.packageId,
        name: mention.name,
        recipients: [actors.alice],
        reason: "mention" as const,
        payload: { author: "bob", excerpt: "@alice, a question" },
    };

    // notify alice as bob, then as carol, who reads no document
    fixture.as("bob");
    await fixture.call(activity.call("notify", { ...fixture.scope, ...notice }));
    fixture.as("carol");
    const refused = fixture.call(activity.call("notify", { ...fixture.scope, ...notice }));

    await expect(refused).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "permission denied: post",
    });
    expect(
        (await fixture.activities()).map((row) => [
            Activity.recipient(row),
            ObjectReference.at("actor", row),
            row.reason,
        ]),
    ).toEqual([[actors.alice, actors.bob, "mention"]]);
});
