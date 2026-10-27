import { ObjectReference } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { activity, focus, notification, subscription } from "../src/index.ts";
import { document, documentParent, review } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("answer a review request from its notification, which breaks through a focus that defers a change until a snooze ends", async () => {
    await using fixture = await TestNotification.open();

    // give bob a phone and a focus for the next two hours, and let him subscribe to a document he edits
    await fixture.browser("bob", "/bob");
    const end = fixture.clock.now() + 120 * 60_000;
    await fixture.set("bob", focus, {
        schedules: [],
        allowed: [],
        isTimeSensitiveAllowed: true,
        until: end,
    });
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
        "bob",
        subscription.call("create", { ...fixture.scope, ...documentParent(plan.id) }),
    );

    // ask bob for approval and change the document, alerting his phone only for the time-sensitive request
    await fixture.call(
        "alice",
        document.call("request", {
            ...fixture.scope,
            id: plan.id,
            reviewer: fixture.subject("bob"),
        }),
    );
    await fixture.call(
        "alice",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Rewrote the intro" }),
    );
    await fixture.dispatch();
    const inbox = await fixture.inbox("bob");
    const { entries } = inbox;
    const request = entries.find((entry) => review.is(entry.activity));
    const change = entries.find((entry) => !review.is(entry.activity));
    if (request === undefined || change === undefined) {
        throw new TypeError("bob lacks the review request or the change");
    }
    const sent = async () =>
        (await fixture.messages("bob", change.notification.id)).map((row) => [
            row.to.channel,
            row.status,
        ]);
    const topic = request.notification.id.slice("notification-".length).replaceAll("-", "");
    expect([
        inbox.unread(),
        fixture.pushes.requests.map((pushed) => [pushed.urgency, pushed.topic]),
        await sent(),
    ]).toEqual([2, [["high", topic]], []]);

    // approve on the activity in the document's space, then read the notification in the home
    const refused = await refusal(
        fixture.call(
            "bob",
            activity.call("act", {
                ...fixture.scope,
                id: request.activity.id,
                action: "approve",
                text: "looks good",
            }),
        ),
    );
    expect(refused).toEqual([
        "BAD_REQUEST",
        "TEXT_UNEXPECTED",
        "action approve of review must take text exactly when it declares text",
    ]);
    await fixture.call(
        "bob",
        activity.call("act", { ...fixture.scope, id: request.activity.id, action: "approve" }),
    );
    await fixture.call(
        "bob",
        notification.call("read", {
            ...fixture.homeScope("bob"),
            id: request.notification.id,
        }),
    );
    await fixture.dispatch();
    expect([
        ObjectReference.at(
            "approvedBy",
            await fixture.call("bob", document.call("get", { ...fixture.scope, id: plan.id })),
        ),
        (
            await fixture.call(
                "bob",
                notification.call("get", {
                    ...fixture.homeScope("bob"),
                    id: request.notification.id,
                }),
            )
        )["readAt"],
        (await fixture.inbox("bob")).unread(),
    ]).toEqual([fixture.subject("bob"), fixture.clock.now(), 1]);

    // snooze the change past the focus, off the badge and the phone
    const until = end + 60 * 60_000;
    await fixture.call(
        "bob",
        notification.call("snooze", {
            ...fixture.homeScope("bob"),
            id: change.notification.id,
            until,
        }),
    );
    const snoozed = (await fixture.inbox("bob")).unread();
    await fixture.wait(120);
    expect([snoozed, fixture.pushes.requests.length, await sent()]).toEqual([0, 1, []]);

    // wake the change once the snooze ends, badging and alerting again
    await fixture.wait(60);
    expect([
        (await fixture.inbox("bob")).unread(),
        fixture.pushes.requests.map((pushed) => pushed.urgency),
        await sent(),
    ]).toEqual([
        1,
        ["high", "normal"],
        [
            ["desktop", "sent"],
            ["email", "sent"],
            ["push", "sent"],
        ],
    ]);
});
