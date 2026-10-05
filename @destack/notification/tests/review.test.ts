import { TEST_DIALECTS } from "@destack/db/test";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { activity, focus, notification, subscription } from "../src/index.ts";
import { document, review } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

/** Bob's phone. */
const PHONE = schema
    .identifier("push-endpoint")
    .parse("push-endpoint-019f5530-8000-7000-8000-00000000000b");

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
        const end = fixture.now() + 120 * 60_000;
        await fixture.set("bob", focus, {
            schedules: [],
            allowed: [],
            isTimeSensitiveAllowed: true,
            until: end,
        });
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "editor",
            subject: actors.bob,
        });
        fixture.as("bob");
        await fixture.call(subscription, "create", documentParent(plan.id));
        const box = fixture.inbox("bob");

        // ask bob for approval and change the document, alerting his phone only for the time-sensitive request
        fixture.as("alice");
        await fixture.call(document, "request", { id: plan.id, reviewer: actors.bob });
        await fixture.call(document, "edit", { id: plan.id, summary: "Rewrote the intro" });
        await box.until((state) => state.unread === 2);
        await fixture.dispatch();
        const rows = [...box.rows.values()];
        const request = rows.find((row) => review.is(row));
        const change = rows.find((row) => !review.is(row));
        if (request === undefined || change === undefined) {
            throw new TypeError("bob lacks the review request or the change");
        }
        expect([
            fixture.pushes.requests.map((pushed) => [pushed.urgency, pushed.topic]),
            (await fixture.deliveries(change.id)).map((row) => [row.channel, row.state, row.dueAt]),
        ]).toEqual([
            [["high", request.id.slice("notification-".length).replaceAll("-", "")]],
            [
                ["email", "skipped", fixture.now()],
                ["push", "pending", end],
            ],
        ]);

        // approve on the activity in the document's space, then read the notification in the home
        fixture.as("bob");
        await expect(
            fixture.call(activity, "act", {
                id: request.source.id,
                action: "approve",
                text: "looks good",
            }),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "action approve of review takes text only where it asks for it",
        });
        await fixture.call(activity, "act", { id: request.source.id, action: "approve" });
        await fixture.call(notification, "read", { id: request.id });
        await box.until((state) => state.unread === 1);
        expect([
            (await fixture.call(document, "get", { id: plan.id }))["approvedBy"],
            (await fixture.call(notification, "get", { id: request.id }))["readAt"],
        ]).toEqual([Subject.key(actors.bob), fixture.now()]);

        // snooze the change past the focus and keep it from the badge and the phone
        const until = end + 60 * 60_000;
        await fixture.call(notification, "snooze", { id: change.id, until });
        await box.until((state) => state.unread === 0);
        fixture.wait(120);
        await fixture.dispatch();
        expect([
            fixture.pushes.requests.length,
            (await fixture.deliveries(change.id)).map((row) => [row.channel, row.state, row.dueAt]),
        ]).toEqual([
            1,
            [
                ["email", "skipped", end - 120 * 60_000],
                ["push", "pending", until],
            ],
        ]);

        // wake the change once the snooze ends, badging and alerting again
        fixture.wait(60);
        await fixture.dispatch();
        await box.until((state) => state.unread === 1);
        expect([
            fixture.pushes.requests.map((pushed) => pushed.urgency),
            (await fixture.deliveries(change.id)).map((row) => [
                row.channel,
                row.state,
                row.reason,
            ]),
        ]).toEqual([
            ["high", "normal"],
            [
                ["email", "skipped", "unaddressed"],
                ["push", "sent", null],
            ],
        ]);
    },
);
