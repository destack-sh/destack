import { nth, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { message } from "@destack/message";
import { focus, Notification } from "../src/index.ts";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("defer a mention through quiet hours, show it on the desktops still signed in once they end, and mark it delivered on receipt", async () => {
    await using fixture = await TestNotification.open();

    // give carol a laptop, the client she signed in on, a studio desktop, and quiet hours from 11:00 to 14:00 in Vienna on Mondays, now noon there
    const laptop = fixture.client("carol");
    const studio = await fixture.desktop("carol");
    await fixture.contact("carol", { timeZone: "Europe/Vienna" });
    await fixture.set("carol", focus, {
        schedules: [{ days: [1], from: "11:00", to: "14:00" }],
        allowed: [],
        isTimeSensitiveAllowed: true,
    });

    // mention carol, whose delivery waits for the end of her quiet hours at 12:00 UTC
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
            subject: fixture.subject("carol"),
        }),
    );
    await fixture.call(
        "alice",
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text: "@carol, a question",
            mentions: [fixture.subject("carol")],
        }),
    );
    await fixture.simulation.idle();
    const mentioned = nth((await fixture.inbox("carol")).entries, 0).notification;
    const end = Date.UTC(2026, 8, 28, 12, 0);
    const { served: home } = await fixture.home("carol");
    const desktops = async () =>
        (await home.database.select().from(message.table))
            .filter((row) => Notification.from(row) === mentioned.id)
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
    const read = async () => (await desktops()).map((row) => [row.client, row.status, row.reason]);
    expect(await read()).toEqual([]);

    // sign carol out of the studio, then show the notification on the laptop once the quiet hours end
    await fixture.revoke("carol", studio);
    await fixture.reach(end);
    expect([fixture.clock.now(), await read()]).toEqual([end, [[laptop, "sent", null]]]);

    // mark the laptop's message delivered as the laptop receives it, twice alike
    const [shown] = (await desktops()).map((row) => row.id);
    const receive = (id: string | undefined) =>
        refusal(
            fixture.call(
                "carol",
                message.call("receive", {
                    ...message.in(fixture.homeScope("carol").spaceId),
                    id: present(id, "a desktop message"),
                }),
            ),
        );
    const first = await receive(shown);
    await fixture.wait(1);
    const again = await receive(shown);
    expect([first, again, await read()]).toEqual(["done", "done", [[laptop, "delivered", null]]]);
});
