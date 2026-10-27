import { nth, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { notification } from "../src/index.ts";
import { decrypt } from "@destack/message/test";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("send mentions to the desktop and the phone, badging the inbox until they are read", async () => {
    await using fixture = await TestNotification.open();

    // give bob a laptop, a phone and an email address, and share two documents with him
    const phone = await fixture.browser("bob", "/bob");
    const laptop = fixture.client("bob");
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    const draft = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Draft" }),
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
        document.call("grant", {
            ...fixture.scope,
            id: draft.id,
            relation: "viewer",
            subject: fixture.subject("bob"),
        }),
    );

    // mention bob while he is away, badging his inbox and alerting his laptop and phone, the email waiting in its run
    await fixture.call(
        "alice",
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text: "@bob, tighten the intro",
            mentions: [fixture.subject("bob")],
        }),
    );
    await fixture.dispatch();
    const first = nth((await fixture.inbox("bob")).entries, 0);
    const home = fixture.homeScope("bob").spaceId;
    expect([
        (await fixture.inbox("bob")).unread(),
        (await fixture.messages("bob", first.notification.id)).map((row) => [
            row.to.channel,
            row.status,
        ]),
        JSON.parse(await decrypt(phone.browser, nth(fixture.pushes.requests, 0).body)),
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
            activity: { scope: fixture.scope.spaceId, id: first.activity.id },
            actions: [{ name: "reply", title: "Reply", isDestructive: false, hasText: true }],
        },
        [["normal", 86_400, 32]],
    ]);

    // mention bob again, alerting his laptop and his phone and holding the email 15 minutes
    await fixture.call(
        "alice",
        document.call("remark", {
            ...fixture.scope,
            id: draft.id,
            text: "@bob, see the draft",
            mentions: [fixture.subject("bob")],
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
            { to: { channel: "desktop", client: laptop }, status: "sent", reason: null },
            {
                to: { channel: "push", client: phone.id, user: fixture.subject("bob").id },
                status: "sent",
                reason: null,
            },
        ],
    ]);

    // read the first, then email the second after 15 unread minutes and send the first no email
    await fixture.call(
        "bob",
        notification.call("read", {
            ...fixture.homeScope("bob"),
            id: first.notification.id,
        }),
    );
    const unread = (await fixture.inbox("bob")).unread();
    await fixture.wait(15);
    expect([
        unread,
        fixture.mails.sent.map((mail) => [mail.to, mail.subject, mail.text, typeof mail.key]),
        ...(await Promise.all(
            [first, second].map(async (entry) =>
                (await fixture.messages("bob", entry.notification.id)).map((row) => row.to.channel),
            ),
        )),
    ]).toEqual([
        1,
        [["bob@example.test", "⁨alice⁩ mentioned you", "⁨@bob, see the draft⁩", "string"]],
        ["desktop", "push"],
        ["desktop", "email", "push"],
    ]);

    // mark both read at once, clearing the badge
    const now = fixture.clock.now();
    const listed = (await fixture.inbox("bob")).entries.map(({ notification: row }) => row);
    const readMany = async (readAt: number) =>
        fixture.call(
            "bob",
            notification.call("readMany", {
                ...fixture.homeScope("bob"),
                items: listed.map(({ id, revision }) => ({ id, revision, readAt })),
            }),
        );
    await readMany(now);
    const inbox = await fixture.inbox("bob");
    expect([inbox.unread(), inbox.entries.map((entry) => entry.notification.readAt)]).toEqual([
        0,
        [now, now],
    ]);

    // refuse a read in the future, and a stale list that would mark them unread again
    expect([await refusal(readMany(now + 1)), await refusal(readMany(now - 1))]).toEqual([
        ["BAD_REQUEST", "READ_IN_FUTURE", "a notification cannot be read in the future"],
        ["CONFLICT", "REVISION_STALE", "notification revision has changed"],
    ]);
});

test("refuse a mention of someone who cannot read the document, notifying nobody", async () => {
    // mention bob and dave on a document only bob views
    await using fixture = await TestNotification.open();
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "viewer",
            subject: fixture.subject("bob"),
        }),
    );
    const refused = await refusal(
        fixture.call(
            "alice",
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@bob @dave, a question",
                mentions: [fixture.subject("bob"), fixture.subject("dave")],
            }),
        ),
    );

    // refuse the remark for dave, and record no activity for bob either
    expect([refused, await fixture.activities()]).toEqual([
        [
            "FORBIDDEN",
            "RECIPIENT_FORBIDDEN",
            `user ${fixture.subject("dave").id} cannot read document ${plan.id}`,
        ],
        [],
    ]);
});
