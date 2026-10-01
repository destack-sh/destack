import { subjectKey } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { focus } from "../src/index.ts";
import { subscribeBrowser } from "./fixture/browser.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "withhold a notification from a recipient who lost access to its source before it went out, on %s",
    async (dialect) => {
        const {
            call,
            homes,
            pushes,
            mails,
            dispatch,
            inbox,
            deliveries,
            wait,
            now,
            notifications,
        } = await serveSpace(dialect);

        // give carol a phone, an email address and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
        const carol = subjectKey(actors.carol);
        const phone = await subscribeBrowser();
        homes.endpoints.set(carol, [
            {
                id: "push-endpoint-019f5530-8000-7000-8000-00000000000c" as never,
                url: "https://push.example/carol",
                keys: phone.keys,
                device: null,
            },
        ]);
        homes.emails.set(carol, "carol@example.com");
        homes.zones.set(subjectKey(actors.carol), "Europe/Vienna");
        homes.set("carol", focus, {
            schedules: [{ days: [1], from: "11:00", to: "14:00" }],
            allowed: [],
            isTimeSensitiveAllowed: true,
        });

        // mention carol on a document she views and defer the alerts until her quiet hours end at 12:00 UTC
        const plan = await call("create", { title: "Launch plan" });
        const { id: relationship } = await call("grant", {
            id: plan.id,
            relation: "viewer",
            subject: actors.carol,
        });
        const box = inbox("carol");
        await call("remark", { id: plan.id, text: "@carol, a question", mentions: [actors.carol] });
        await box.until((state) => state.unread === 1);
        await dispatch();
        const [mentioned] = await notifications();
        const end = Date.UTC(2026, 8, 28, 12, 0);
        expect(
            (await deliveries(mentioned!.id)).map((row) => [row.channel, row.state, row.dueAt]),
        ).toEqual([
            ["email", "pending", end],
            ["push", "pending", end],
        ]);

        // revoke carol's access to empty her inbox and badge at once
        await call("revoke", { id: plan.id, relationshipId: relationship });
        await box.until((state) => state.rows.size === 0 && state.unread === 0);

        // send nothing after the quiet hours end and keep the notification for a regained share
        wait(120);
        await dispatch();
        expect([
            now(),
            pushes.requests.length,
            mails.sent.length,
            (await deliveries(mentioned!.id)).map((row) => [row.channel, row.state, row.reason]),
            (await notifications()).map((row) => [row.id, row.readAt]),
        ]).toEqual([
            end,
            0,
            0,
            [
                ["email", "skipped", "withheld"],
                ["push", "skipped", "withheld"],
            ],
            [[mentioned!.id, null]],
        ]);
    },
);
