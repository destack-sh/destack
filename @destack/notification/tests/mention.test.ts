import { subjectKey } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { decide, focus, notification, type NotificationRow, summary } from "../src/index.ts";
import { decrypt, subscribeBrowser } from "./fixture/browser.ts";
import { mention, notes } from "./fixture/document.ts";
import { actors, type Homes, serveSpace } from "./fixture/space.ts";

/** Bob's phone, a browser holding a push subscription. */
const PHONE = "push-endpoint-019f5530-8000-7000-8000-00000000000b";

test.for(TEST_DIALECTS)(
    "deliver mentions to the desktop and the phone, badging the inbox until they are read, on %s",
    async (dialect) => {
        const { call, as, homes, pushes, mails, dispatch, inbox, deliveries, wait, now, spaceId } =
            await serveSpace(dialect);

        // give bob a phone and an email address, and share two documents with him
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
        const plan = await call("create", { title: "Launch plan" });
        const draft = await call("create", { title: "Draft" });
        await call("grant", { id: plan.id, relation: "editor", subject: actors.bob });
        await call("grant", { id: draft.id, relation: "viewer", subject: actors.bob });
        const box = inbox("bob");

        // mention bob while he is away, badging his inbox and alerting his desktop and phone
        await call("remark", {
            id: plan.id,
            text: "@bob, tighten the intro",
            mentions: [actors.bob],
        });
        await box.until((held) => held.unread === 1);
        await dispatch();
        const [first] = [...box.rows.values()];
        expect([
            box.unread,
            await banner(homes, first!, spaceId, now()),
            JSON.parse(await decrypt(phone, pushes.requests[0]!.body)),
            pushes.requests.map((request) => [request.urgency, request.ttl, request.topic.length]),
        ]).toEqual([
            1,
            { title: "alice mentioned you", body: "@bob, tighten the intro" },
            {
                notification: first!.id,
                scope: spaceId,
                content: { title: "alice mentioned you", body: "@bob, tighten the intro" },
                actions: [{ name: "reply", title: "Reply", isDestructive: false, hasText: true }],
            },
            [["normal", 86_400, 32]],
        ]);

        // mention bob again while he works at his desktop, leaving his phone quiet
        homes.active.set(bob, ["device-desktop"]);
        await call("remark", { id: draft.id, text: "@bob, see the draft", mentions: [actors.bob] });
        await box.until((held) => held.unread === 2);
        await dispatch();
        const second = [...box.rows.values()].find((row) => row.id !== first!.id)!;
        expect([pushes.requests.length, await deliveries(second.id)]).toEqual([
            1,
            [
                {
                    channel: "email",
                    endpoint: null,
                    state: "pending",
                    dueAt: now() + 15 * 60_000,
                    isSummarized: false,
                    reason: null,
                },
                {
                    channel: "push",
                    endpoint: PHONE,
                    state: "skipped",
                    dueAt: now(),
                    isSummarized: false,
                    reason: "present",
                },
            ],
        ]);

        // read the first on the desktop, then email only the second once it stayed unread 15 minutes
        as("bob");
        await call("read", { id: first!.id }, notification);
        await box.until((held) => held.unread === 1);
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
        await box.until((held) => held.unread === 0);
        expect([box.rows.get(first!.id)!.readAt, box.rows.get(second.id)!.readAt]).toEqual([
            readAt,
            now(),
        ]);
    },
);

/** Decide a desktop banner for an inbox row as the desktop does, returning its text, absent for a quiet row. */
async function banner(homes: Homes, row: NotificationRow, space: string, now: number) {
    // resolve the recipient's settings for the notification's package and space
    const settings = await homes
        .settings(actors.bob, { space, package: notes.id })
        .resolve({ preference: mention.preference, focus, summary });

    // show a banner where the desktop channel sends at once
    const decision = decide({
        channel: "desktop",
        notification: row,
        interruption: mention.definition.interruption,
        preference: settings.preference.value,
        focus: settings.focus.value,
        summary: settings.summary.value,
        timeZone: await homes.timeZone(actors.bob),
        isReadable: true,
        isAddressed: true,
        isPresent: false,
        isSummaryDue: false,
        now,
    });

    return decision.action === "send" ? mention.render(row) : undefined;
}
