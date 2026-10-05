import { TEST_DIALECTS } from "@destack/db/test";
import { aligned, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { focus, notification } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Carol's laptop. */
const LAPTOP = schema.identifier("device").parse("device-019f5530-8000-7000-8000-00000000000d");

/** Carol's studio machine that she signs out of. */
const STUDIO = schema.identifier("device").parse("device-019f5530-8000-7000-8000-00000000000e");

test.for(TEST_DIALECTS)(
    "defer banners for each desktop through quiet hours, then send them to the desktops still signed in, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give carol two desktops and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
        await fixture.desktop("carol", LAPTOP);
        await fixture.desktop("carol", STUDIO);
        await fixture.reach("carol", { timeZone: "Europe/Vienna" });
        await fixture.set("carol", focus, {
            schedules: [{ days: [1], from: "11:00", to: "14:00" }],
            allowed: [],
            isTimeSensitiveAllowed: true,
        });

        // mention carol and defer one delivery per desktop until her quiet hours end at 12:00 UTC
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        await fixture.call(document, "grant", {
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
        const read = async () =>
            (await fixture.deliveries(mentioned.id))
                .filter((row) => row.channel === "desktop")
                .map((row) => [row.device, row.state, row.dueAt, row.reason]);
        expect(await read()).toEqual([
            [LAPTOP, "pending", end, null],
            [STUDIO, "pending", end, null],
        ]);

        // sign carol out of the studio, then show the notification on the laptop once the quiet hours end
        await fixture.desktop("carol", STUDIO, true);
        fixture.wait(120);
        await fixture.dispatch();
        expect([fixture.now(), await read()]).toEqual([
            end,
            [
                [LAPTOP, "sent", end, null],
                [STUDIO, "skipped", end, "unaddressed"],
            ],
        ]);

        // withdraw the banners once carol reads the notification
        fixture.as("carol");
        await fixture.call(notification, "read", { id: mentioned.id });
        await fixture.dispatch();
        expect(await read()).toEqual([]);
    },
);
