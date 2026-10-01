import { subjectKey } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { notification } from "../src/index.ts";
import { decrypt, subscribeBrowser } from "./fixture/browser.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Bob's phone, a browser with a push subscription. */
const PHONE = "push-endpoint-019f5530-8000-7000-8000-00000000000b";

/** Bob's laptop with a desktop that shows his banners. */
const LAPTOP = "device-019f5530-8000-7000-8000-00000000000c";

/** The reply a mention's banner offers. */
const REPLY = {
    name: "reply",
    title: "Reply",
    isDestructive: false,
    text: { placeholder: "Reply", button: "Send" },
};

test.for(TEST_DIALECTS)(
    "deliver mentions to the desktop and the phone, badging the inbox until they are read, on %s",
    async (dialect) => {
        const { call, as, homes, pushes, mails, dispatch, inbox, deliveries, wait, now, spaceId } =
            await serveSpace(dialect);

        // give bob a laptop, a phone and an email address, and share two documents with him
        const phone = await subscribeBrowser();
        const bob = subjectKey(actors.bob);
        homes.endpoints.set(bob, [
            {
                id: PHONE as never,
                url: "https://fcm.googleapis.com/fcm/send/bob",
                keys: phone.keys,
                device: "device-phone" as never,
            },
        ]);
        homes.emails.set(bob, "bob@example.com");
        homes.desktops.set(bob, [LAPTOP]);
        const plan = await call("create", { title: "Launch plan" });
        const draft = await call("create", { title: "Draft" });
        await call("grant", { id: plan.id, relation: "editor", subject: actors.bob });
        await call("grant", { id: draft.id, relation: "viewer", subject: actors.bob });
        const box = inbox("bob");

        // mention bob while he is away, badging his inbox and alerting his laptop and phone
        await call("remark", {
            id: plan.id,
            text: "@bob, tighten the intro",
            mentions: [actors.bob],
        });
        await box.until((state) => state.unread === 1);
        await dispatch();
        const [first] = [...box.rows.values()];
        const [desktop] = await deliveries(first!.id);
        expect([
            box.unread,
            [desktop!.channel, desktop!.device, desktop!.state, desktop!.banner],
            JSON.parse(await decrypt(phone, pushes.requests[0]!.body)),
            pushes.requests.map((request) => [request.urgency, request.ttl, request.topic.length]),
        ]).toEqual([
            1,
            [
                "desktop",
                LAPTOP,
                "sent",
                { title: "alice mentioned you", body: "@bob, tighten the intro", actions: [REPLY] },
            ],
            {
                notification: first!.id,
                scope: spaceId,
                content: { title: "alice mentioned you", body: "@bob, tighten the intro" },
                actions: [{ name: "reply", title: "Reply", isDestructive: false, hasText: true }],
            },
            [["normal", 86_400, 32]],
        ]);

        // mention bob again while he works at his laptop and leave his phone quiet
        homes.active.set(bob, [LAPTOP]);
        await call("remark", { id: draft.id, text: "@bob, see the draft", mentions: [actors.bob] });
        await box.until((state) => state.unread === 2);
        await dispatch();
        const second = [...box.rows.values()].find((row) => row.id !== first!.id)!;
        expect([pushes.requests.length, await deliveries(second.id)]).toEqual([
            1,
            [
                {
                    channel: "desktop",
                    endpoint: null,
                    device: LAPTOP,
                    banner: {
                        title: "alice mentioned you",
                        body: "@bob, see the draft",
                        actions: [REPLY],
                    },
                    state: "sent",
                    dueAt: now(),
                    isSummarized: false,
                    reason: null,
                },
                {
                    channel: "email",
                    endpoint: null,
                    device: null,
                    banner: null,
                    state: "pending",
                    dueAt: now() + 15 * 60_000,
                    isSummarized: false,
                    reason: null,
                },
                {
                    channel: "push",
                    endpoint: PHONE,
                    device: null,
                    banner: null,
                    state: "skipped",
                    dueAt: now(),
                    isSummarized: false,
                    reason: "present",
                },
            ],
        ]);

        // read the first on the desktop to withdraw its banner, then email the second after 15 unread minutes
        as("bob");
        await call("read", { id: first!.id }, notification);
        await box.until((state) => state.unread === 1);
        wait(15);
        await dispatch();
        expect([
            mails.sent.map((mail) => [mail.to, mail.subject, mail.text, typeof mail.key]),
            (await deliveries(first!.id)).map((row) => [row.channel, row.state, row.reason]),
        ]).toEqual([
            [["bob@example.com", "alice mentioned you", "@bob, see the draft", "string"]],
            [
                ["email", "skipped", "read"],
                ["push", "sent", null],
            ],
        ]);

        // mark every unread notification read at once, clearing the badge and keeping the first one's time
        const readAt = now() - 15 * 60_000;
        await call("readAll", { where: { readAt: null }, readAt: now() }, notification);
        await box.until((state) => state.unread === 0);
        expect([box.rows.get(first!.id)!.readAt, box.rows.get(second.id)!.readAt]).toEqual([
            readAt,
            now(),
        ]);
    },
);
