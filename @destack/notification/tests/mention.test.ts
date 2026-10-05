import { TEST_DIALECTS } from "@destack/db/test";
import { aligned, found, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification } from "../src/index.ts";
import { decrypt, subscribeBrowser } from "@destack/message/test";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Bob's phone, a browser with a push subscription. */
const PHONE = schema
    .identifier("push-endpoint")
    .parse("push-endpoint-019f5530-8000-7000-8000-00000000000b");

/** Bob's laptop with a desktop that shows his notifications. */
const LAPTOP = schema.identifier("device").parse("device-019f5530-8000-7000-8000-00000000000c");

test.for(TEST_DIALECTS)(
    "deliver mentions to the desktop and the phone, badging the inbox until they are read, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give bob a laptop, a phone and an email address, and share two documents with him
        const phone = await subscribeBrowser();
        await fixture.subscribe("bob", {
            id: PHONE,
            url: "https://fcm.googleapis.com/fcm/send/bob",
            keys: phone.keys,
        });
        await fixture.reach("bob", { email: "bob@example.com" });
        await fixture.desktop("bob", LAPTOP);
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        const draft = await fixture.call(document, "create", { title: "Draft" });
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "editor",
            subject: actors.bob,
        });
        await fixture.call(document, "grant", {
            id: draft.id,
            relation: "viewer",
            subject: actors.bob,
        });
        const box = fixture.inbox("bob");

        // mention bob while he is away, badging his inbox and alerting his laptop and phone
        await fixture.call(document, "remark", {
            id: plan.id,
            text: "@bob, tighten the intro",
            mentions: [actors.bob],
        });
        await box.until((state) => state.unread === 1);
        await fixture.dispatch();
        const first = aligned([...box.rows.values()], 0);
        const desktop = aligned(await fixture.deliveries(first.id), 0);
        expect([
            box.unread,
            [desktop.channel, desktop.device, desktop.state],
            JSON.parse(await decrypt(phone, aligned(fixture.pushes.requests, 0).body)),
            fixture.pushes.requests.map((request) => [
                request.urgency,
                request.ttl,
                request.topic.length,
            ]),
        ]).toEqual([
            1,
            ["desktop", LAPTOP, "sent"],
            {
                notification: first.id,
                activity: first.source,
                scope: fixture.homeId,
                content: { title: "alice mentioned you", body: "@bob, tighten the intro" },
                actions: [{ name: "reply", title: "Reply", isDestructive: false, hasText: true }],
            },
            [["normal", 86_400, 32]],
        ]);

        // mention bob again, alerting his laptop and his phone
        await fixture.call(document, "remark", {
            id: draft.id,
            text: "@bob, see the draft",
            mentions: [actors.bob],
        });
        await box.until((state) => state.unread === 2);
        await fixture.dispatch();
        const second = [...box.rows.values()].find((row) => row.id !== first.id);
        if (second === undefined) {
            throw new TypeError("bob has no second notification");
        }
        expect([fixture.pushes.requests.length, await fixture.deliveries(second.id)]).toEqual([
            2,
            [
                {
                    channel: "desktop",
                    endpoint: null,
                    device: LAPTOP,
                    state: "sent",
                    dueAt: fixture.now(),
                    isSummarized: false,
                    reason: null,
                },
                {
                    channel: "email",
                    endpoint: null,
                    device: null,
                    state: "pending",
                    dueAt: fixture.now() + 15 * 60_000,
                    isSummarized: false,
                    reason: null,
                },
                {
                    channel: "push",
                    endpoint: PHONE,
                    device: null,
                    state: "sent",
                    dueAt: fixture.now(),
                    isSummarized: false,
                    reason: null,
                },
            ],
        ]);

        // read the first on the desktop to withdraw it there, then email the second after 15 unread minutes
        fixture.as("bob");
        await fixture.call(notification, "read", { id: first.id });
        await box.until((state) => state.unread === 1);
        fixture.wait(15);
        await fixture.dispatch();
        expect([
            fixture.mails.sent.map((mail) => [mail.to, mail.subject, mail.text, typeof mail.key]),
            (await fixture.deliveries(first.id)).map((row) => [row.channel, row.state, row.reason]),
        ]).toEqual([
            [["bob@example.com", "alice mentioned you", "@bob, see the draft", "string"]],
            [
                ["email", "skipped", "read"],
                ["push", "sent", null],
            ],
        ]);

        // mark every unread notification read at once, clearing the badge and keeping the first one's time
        const readAt = fixture.now() - 15 * 60_000;
        await fixture.call(notification, "readAll", {
            where: { readAt: null },
            readAt: fixture.now(),
        });
        await box.until((state) => state.unread === 0);
        expect([found(box.rows, first.id).readAt, found(box.rows, second.id).readAt]).toEqual([
            readAt,
            fixture.now(),
        ]);
    },
);
