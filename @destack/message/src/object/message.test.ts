import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Destination } from "./message.ts";

/** A signed-in desktop client. */
const LAPTOP = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000a2");

test("key each destination by its channel and address, matching one address across its other fields", () => {
    // key one destination per channel, and the same browser under rotated push keys
    const browser = { channel: "push" as const, url: "https://push.example/phone" };
    const destinations: readonly Destination[] = [
        { channel: "desktop", client: LAPTOP },
        { channel: "email", address: "dana@example.com" },
        { ...browser, keys: { p256dh: "old", auth: "old" } },
        { channel: "webhook" },
    ];
    const rotated: Destination = { ...browser, keys: { p256dh: "new", auth: "new" } };

    // expect one key per destination, and the rotated browser the same as before
    expect([
        destinations.map((to) => Destination.key(to)),
        Destination.same(rotated, { ...browser, keys: { p256dh: "old", auth: "old" } }),
        Destination.same(rotated, { channel: "email", address: "dana@example.com" }),
    ]).toEqual([
        [
            `desktop:${LAPTOP}`,
            "email:dana@example.com",
            "push:https://push.example/phone",
            "webhook",
        ],
        true,
        false,
    ]);
});
