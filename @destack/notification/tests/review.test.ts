import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { focus, notification, subscription } from "../src/index.ts";
import { document, review } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Bob's phone. */
const PHONE = "push-endpoint-019f5530-8000-7000-8000-00000000000b";

test.for(TEST_DIALECTS)(
    "answer a review request from its notification, which breaks through a focus that defers a change until a snooze ends, on %s",
    async (dialect) => {
        const { call, as, homes, pushes, dispatch, inbox, deliveries, wait, now, host } =
            await serveSpace(dialect);

        // give bob a phone and a focus for the next two hours, and let him subscribe to a document he edits
        const bob = Subject.key(actors.bob);
        homes.endpoints.set(bob, [
            {
                id: PHONE as never,
                url: "https://push.example/bob",
                keys: {
                    p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
                    auth: "BTBZMqHH6r4Tts7J_aSIgg",
                },
                device: null,
            },
        ]);
        const end = now() + 120 * 60_000;
        homes.set("bob", focus, {
            schedules: [],
            allowed: [],
            isTimeSensitiveAllowed: true,
            until: end,
        });
        const plan = await call("create", { title: "Launch plan" });
        await call("grant", { id: plan.id, relation: "editor", subject: actors.bob });
        as("bob");
        await call("create", host(plan.id), subscription);
        const box = inbox("bob");

        // ask bob for approval and change the document, alerting his phone only for the time-sensitive request
        as("alice");
        await call("request", { id: plan.id, reviewer: actors.bob });
        await call("edit", { id: plan.id, summary: "Rewrote the intro" });
        await box.until((state) => state.unread === 2);
        await dispatch();
        const rows = [...box.rows.values()];
        const request = rows.find((row) => review.is(row))!;
        const change = rows.find((row) => !review.is(row))!;
        expect([
            pushes.requests.map((pushed) => [pushed.urgency, pushed.topic]),
            (await deliveries(change.id)).map((row) => [row.channel, row.state, row.dueAt]),
        ]).toEqual([
            [["high", request.id.slice("notification-".length).replaceAll("-", "")]],
            [
                ["email", "skipped", now()],
                ["push", "pending", end],
            ],
        ]);

        // approve from the notification on the server: the action's call and reading it, as one call
        as("bob");
        await expect(
            call("act", { id: request.id, action: "approve", text: "looks good" }, notification),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "action approve of review takes text only where it asks for it",
        });
        await call("act", { id: request.id, action: "approve" }, notification);
        await box.until((state) => state.unread === 1);
        expect([
            (await call("get", { id: plan.id }, document)).approvedBy,
            (await call("get", { id: request.id }, notification)).readAt,
        ]).toEqual([Subject.key(actors.bob), now()]);

        // snooze the change past the focus and keep it from the badge and the phone
        const until = end + 60 * 60_000;
        await call("snooze", { id: change.id, until }, notification);
        await box.until((state) => state.unread === 0);
        wait(120);
        await dispatch();
        expect([
            pushes.requests.length,
            (await deliveries(change.id)).map((row) => [row.channel, row.state, row.dueAt]),
        ]).toEqual([
            1,
            [
                ["email", "skipped", end - 120 * 60_000],
                ["push", "pending", until],
            ],
        ]);

        // wake the change once the snooze ends, badging and alerting again
        wait(60);
        await dispatch();
        await box.until((state) => state.unread === 1);
        expect([
            pushes.requests.map((pushed) => pushed.urgency),
            (await deliveries(change.id)).map((row) => [row.channel, row.state, row.reason]),
        ]).toEqual([
            ["high", "normal"],
            [
                ["email", "skipped", "unaddressed"],
                ["push", "sent", null],
            ],
        ]);
    },
);
