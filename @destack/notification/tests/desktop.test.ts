import { subjectKey } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { focus, notification } from "../src/index.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Carol's laptop. */
const LAPTOP = "device-019f5530-8000-7000-8000-00000000000d";

/** Carol's studio machine that she signs out of. */
const STUDIO = "device-019f5530-8000-7000-8000-00000000000e";

test.for(TEST_DIALECTS)(
    "defer banners for each desktop through quiet hours, then send them to the desktops still signed in, on %s",
    async (dialect) => {
        const { call, as, homes, dispatch, inbox, deliveries, wait, now, notifications } =
            await serveSpace(dialect);

        // give carol two desktops and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
        const carol = subjectKey(actors.carol);
        homes.desktops.set(carol, [LAPTOP, STUDIO]);
        homes.zones.set(carol, "Europe/Vienna");
        homes.set("carol", focus, {
            schedules: [{ days: [1], from: "11:00", to: "14:00" }],
            allowed: [],
            isTimeSensitiveAllowed: true,
        });

        // mention carol and defer one delivery per desktop until her quiet hours end at 12:00 UTC
        const plan = await call("create", { title: "Launch plan" });
        await call("grant", { id: plan.id, relation: "viewer", subject: actors.carol });
        const box = inbox("carol");
        await call("remark", { id: plan.id, text: "@carol, a question", mentions: [actors.carol] });
        await box.until((state) => state.unread === 1);
        await dispatch();
        const [mentioned] = await notifications();
        const end = Date.UTC(2026, 8, 28, 12, 0);
        const read = async () =>
            (await deliveries(mentioned!.id))
                .filter((row) => row.channel === "desktop")
                .map((row) => [row.device, row.state, row.dueAt, row.reason, row.banner?.title]);
        expect(await read()).toEqual([
            [LAPTOP, "pending", end, null, undefined],
            [STUDIO, "pending", end, null, undefined],
        ]);

        // sign carol out of the studio, then send the laptop's banner once the quiet hours end
        homes.desktops.set(carol, [LAPTOP]);
        wait(120);
        await dispatch();
        expect([now(), await read()]).toEqual([
            end,
            [
                [LAPTOP, "sent", end, null, "alice mentioned you"],
                [STUDIO, "skipped", end, "unaddressed", undefined],
            ],
        ]);

        // withdraw the banners once carol reads the notification
        as("carol");
        await call("read", { id: mentioned!.id }, notification);
        await dispatch();
        expect(await read()).toEqual([]);
    },
);
