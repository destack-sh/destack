import { TEST_DIALECTS } from "@destack/db/test";
import { nth, present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification } from "../src/index.ts";
import { decrypt, subscribeBrowser } from "@destack/message/test";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Bob's phone, a browser with a push subscription. */
const PHONE = schema.identifier("client").parse("client-019f5530-8000-7000-8000-00000000000b");

/** Bob's laptop with a desktop that shows his notifications. */
const LAPTOP = schema.identifier("client").parse("client-019f5530-8000-7000-8000-00000000000c");

test.for(TEST_DIALECTS)(
    "send mentions to the desktop and the phone, badging the inbox until they are read, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give bob a laptop, a phone and an email address, and share two documents with him
        const phone = await subscribeBrowser();
        const url = "https://fcm.googleapis.com/fcm/send/bob";
        await fixture.subscribe("bob", { id: PHONE, url, keys: phone.keys });
        await fixture.contact("bob", { email: "bob@example.com" });
        await fixture.desktop("bob", LAPTOP);
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        const draft = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Draft" }),
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
            document.call("grant", {
                ...fixture.scope,
                id: draft.id,
                relation: "viewer",
                subject: actors.bob,
            }),
        );

        // mention bob while he is away, badging his inbox and alerting his laptop and phone, the email waiting in its run
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@bob, tighten the intro",
                mentions: [actors.bob],
            }),
        );
        await fixture.dispatch();
        const first = nth((await fixture.inbox("bob")).entries, 0);
        const home = (await fixture.home("bob")).id;
        expect([
            (await fixture.inbox("bob")).unread(),
            (await fixture.messages("bob", first.notification.id)).map((row) => [
                row.to.channel,
                row.status,
            ]),
            JSON.parse(await decrypt(phone, nth(fixture.pushes.requests, 0).body)),
            fixture.pushes.requests.map((request) => [
                request.urgency,
                request.ttl,
                request.topic.length,
            ]),
        ]).toEqual([
            1,
            [
                ["desktop", "sent"],
                ["push", "sent"],
            ],
            {
                notification: first.notification.id,
                scope: home,
                content: {
                    title: "⁨alice⁩ mentioned you",
                    body: "⁨@bob, tighten the intro⁩",
                },
                activity: { scope: fixture.spaceId, id: first.activity.id },
                actions: [{ name: "reply", title: "Reply", isDestructive: false, hasText: true }],
            },
            [["normal", 86_400, 32]],
        ]);

        // mention bob again, alerting his laptop and his phone and holding the email 15 minutes
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: draft.id,
                text: "@bob, see the draft",
                mentions: [actors.bob],
            }),
        );
        await fixture.dispatch();
        const second = present(
            (await fixture.inbox("bob")).entries.find(
                (entry) => entry.notification.id !== first.notification.id,
            ),
            "the second notification",
        );
        expect([
            (await fixture.inbox("bob")).unread(),
            fixture.pushes.requests.length,
            await fixture.messages("bob", second.notification.id),
        ]).toEqual([
            2,
            2,
            [
                { to: { channel: "desktop", client: LAPTOP }, status: "sent", reason: null },
                { to: { channel: "push", url, keys: phone.keys }, status: "sent", reason: null },
            ],
        ]);

        // read the first, then email the second after 15 unread minutes and send the first no email
        fixture.as("bob");
        await fixture.call(
            notification.call("read", {
                ...(await fixture.homeScope()),
                id: first.notification.id,
            }),
        );
        const unread = (await fixture.inbox("bob")).unread();
        await fixture.wait(15);
        await fixture.dispatch();
        expect([
            unread,
            fixture.mails.sent.map((mail) => [mail.to, mail.subject, mail.text, typeof mail.key]),
            ...(await Promise.all(
                [first, second].map(async (entry) =>
                    (await fixture.messages("bob", entry.notification.id)).map(
                        (row) => row.to.channel,
                    ),
                ),
            )),
        ]).toEqual([
            1,
            [["bob@example.com", "⁨alice⁩ mentioned you", "⁨@bob, see the draft⁩", "string"]],
            ["desktop", "push"],
            ["desktop", "email", "push"],
        ]);

        // mark every notification read at once, clearing the badge
        await fixture.call(
            notification.call("readAll", {
                ...(await fixture.homeScope()),
                where: {},
                readAt: fixture.clock.now(),
            }),
        );
        const inbox = await fixture.inbox("bob");
        const { entries } = inbox;
        expect([inbox.unread(), entries.map((entry) => entry.notification.readAt)]).toEqual([
            0,
            [fixture.clock.now(), fixture.clock.now()],
        ]);
    },
);
