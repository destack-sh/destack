import { TEST_DIALECTS } from "@destack/db/test";
import { nth, present, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { message } from "@destack/message";
import { focus, Notification } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Carol's laptop. */
const LAPTOP = schema.identifier("client").parse("client-019f5530-8000-7000-8000-00000000000d");

/** Carol's studio machine that she signs out of. */
const STUDIO = schema.identifier("client").parse("client-019f5530-8000-7000-8000-00000000000e");

test.for(TEST_DIALECTS)(
    "hold a mention through quiet hours, show it on the desktops still signed in once they end, and mark it delivered on receipt, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give carol two desktops and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
        await fixture.desktop("carol", LAPTOP);
        await fixture.desktop("carol", STUDIO);
        await fixture.contact("carol", { timeZone: "Europe/Vienna" });
        await fixture.set("carol", focus, {
            schedules: [{ days: [1], from: "11:00", to: "14:00" }],
            allowed: [],
            isTimeSensitiveAllowed: true,
        });

        // mention carol, whose delivery waits for the end of her quiet hours at 12:00 UTC
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "viewer",
                subject: actors.carol,
            }),
        );
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@carol, a question",
                mentions: [actors.carol],
            }),
        );
        await fixture.dispatch();
        const mentioned = nth((await fixture.inbox("carol")).entries, 0).notification;
        const end = Date.UTC(2026, 8, 28, 12, 0);
        const { test: home } = await fixture.home("carol");
        const desktops = async () =>
            (await home.database.select().from(message.table))
                .filter((row) => Notification.read(row) === mentioned.id)
                .flatMap((row) =>
                    row.to.channel === "desktop"
                        ? [
                              {
                                  id: row.id,
                                  client: row.to.client,
                                  status: row.status,
                                  reason: row.reason,
                              },
                          ]
                        : [],
                )
                .toSorted((left, right) => left.client.localeCompare(right.client));
        const read = async () =>
            (await desktops()).map((row) => [row.client, row.status, row.reason]);
        expect(await read()).toEqual([]);

        // sign carol out of the studio, then show the notification on the laptop once the quiet hours end
        await fixture.desktop("carol", STUDIO, true);
        await fixture.wait(120);
        await fixture.dispatch();
        expect([fixture.clock.now(), await read()]).toEqual([end, [[LAPTOP, "sent", null]]]);

        // mark the laptop's message delivered as it receives it, twice alike
        fixture.as("carol");
        const [laptop] = (await desktops()).map((row) => row.id);
        const carolHome = await fixture.homeScope();
        const receive = (id: string | undefined) =>
            refusal(
                fixture.call(
                    message.call("receive", {
                        ...message.in(carolHome.spaceId),
                        id: present(id, "a desktop message"),
                    }),
                ),
            );
        const first = await receive(laptop);
        await fixture.wait(1);
        const again = await receive(laptop);
        expect([first, again, await read()]).toEqual([
            "done",
            "done",
            [[LAPTOP, "delivered", null]],
        ]);
    },
);
