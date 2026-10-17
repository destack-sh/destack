import { TEST_DIALECTS } from "@destack/db/test";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { activity, focus, notification, subscription } from "../src/index.ts";
import { document, review } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

/** Bob's phone. */
const PHONE = schema.identifier("client").parse("client-019f5530-8000-7000-8000-00000000000b");

test.for(TEST_DIALECTS)(
    "answer a review request from its notification, which breaks through a focus that defers a change until a snooze ends, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give bob a phone and a focus for the next two hours, and let him subscribe to a document he edits
        await fixture.subscribe("bob", {
            id: PHONE,
            url: "https://push.example/bob",
            keys: {
                p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
                auth: "BTBZMqHH6r4Tts7J_aSIgg",
            },
        });
        const end = fixture.clock.now() + 120 * 60_000;
        await fixture.set("bob", focus, {
            schedules: [],
            allowed: [],
            isTimeSensitiveAllowed: true,
            until: end,
        });
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "editor",
                subject: actors.bob,
            }),
        );
        fixture.as("bob");
        await fixture.call(
            subscription.call("create", { ...fixture.scope, ...documentParent(plan.id) }),
        );

        // ask bob for approval and change the document, alerting his phone only for the time-sensitive request
        fixture.as("alice");
        await fixture.call(
            document.call("request", { ...fixture.scope, id: plan.id, reviewer: actors.bob }),
        );
        await fixture.call(
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
        fixture.as("bob");
        const refused = await refusal(
            fixture.call(
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
            "action approve of review takes text only where it asks for it",
        ]);
        await fixture.call(
            activity.call("act", { ...fixture.scope, id: request.activity.id, action: "approve" }),
        );
        await fixture.call(
            notification.call("read", {
                ...(await fixture.homeScope()),
                id: request.notification.id,
            }),
        );
        await fixture.dispatch();
        expect([
            ObjectReference.at(
                "approvedBy",
                await fixture.call(document.call("get", { ...fixture.scope, id: plan.id })),
            ),
            (
                await fixture.call(
                    notification.call("get", {
                        ...(await fixture.homeScope()),
                        id: request.notification.id,
                    }),
                )
            )["readAt"],
            (await fixture.inbox("bob")).unread(),
        ]).toEqual([actors.bob, fixture.clock.now(), 1]);

        // snooze the change past the focus and keep it from the badge and the phone
        const until = end + 60 * 60_000;
        await fixture.call(
            notification.call("snooze", {
                ...(await fixture.homeScope()),
                id: change.notification.id,
                until,
            }),
        );
        const snoozed = (await fixture.inbox("bob")).unread();
        await fixture.wait(120);
        await fixture.dispatch();
        expect([snoozed, fixture.pushes.requests.length, await sent()]).toEqual([0, 1, []]);

        // wake the change once the snooze ends, badging and alerting again
        await fixture.wait(60);
        await fixture.dispatch();
        expect([
            (await fixture.inbox("bob")).unread(),
            fixture.pushes.requests.map((pushed) => pushed.urgency),
            await sent(),
        ]).toEqual([1, ["high", "normal"], [["push", "sent"]]]);
    },
);
