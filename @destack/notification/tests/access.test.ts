import { TEST_DIALECTS } from "@destack/db/test";
import { aligned, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { focus } from "../src/index.ts";
import { subscribeBrowser } from "@destack/message/test";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "retract a notification from a recipient who loses access to its activity before it goes out, and project it again on a regained share, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give carol a phone, an email address and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
        const phone = await subscribeBrowser();
        await fixture.reach("carol", { email: "carol@example.com", timeZone: "Europe/Vienna" });
        await fixture.subscribe("carol", {
            id: schema
                .identifier("push-endpoint")
                .parse("push-endpoint-019f5530-8000-7000-8000-00000000000c"),
            url: "https://push.example/carol",
            keys: phone.keys,
        });
        await fixture.set("carol", focus, {
            schedules: [{ days: [1], from: "11:00", to: "14:00" }],
            allowed: [],
            isTimeSensitiveAllowed: true,
        });

        // mention carol on a document she views and defer the alerts until her quiet hours end at 12:00 UTC
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        const { id: relationship } = await fixture.call(document, "grant", {
            id: plan.id,
            relation: "viewer",
            subject: actors.carol,
        });
        const box = fixture.inbox("carol");
        await fixture.call(document, "remark", {
            id: plan.id,
            text: "@carol, a question",
            mentions: [actors.carol],
        });
        await box.until((state) => state.unread === 1);
        await fixture.dispatch();
        const mentioned = aligned(await fixture.notifications(), 0);
        const end = Date.UTC(2026, 8, 28, 12, 0);
        expect(
            (await fixture.deliveries(mentioned.id)).map((row) => [
                row.channel,
                row.state,
                row.dueAt,
            ]),
        ).toEqual([
            ["email", "pending", end],
            ["push", "pending", end],
        ]);

        // revoke carol's access to empty her inbox and badge at once
        await fixture.call(document, "revoke", { id: plan.id, relationshipId: relationship });
        await box.until((state) => state.rows.size === 0 && state.unread === 0);

        // send nothing after the quiet hours end, with the notification and its deliveries retracted
        fixture.wait(120);
        await fixture.dispatch();
        expect([
            fixture.now(),
            fixture.pushes.requests.length,
            fixture.mails.sent.length,
            await fixture.deliveries(mentioned.id),
            await fixture.notifications(),
        ]).toEqual([end, 0, 0, [], []]);

        // project the kept activity again once carol regains her share
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "viewer",
            subject: actors.carol,
        });
        await box.until((state) => state.unread === 1);
        expect((await fixture.notifications()).map((row) => [row.source, row.readAt])).toEqual([
            [mentioned.source, null],
        ]);
    },
);
