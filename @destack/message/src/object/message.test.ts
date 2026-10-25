import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Destination } from "./message.ts";

/** A signed-in desktop client. */
const LAPTOP = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000a2");

/** The person's phone. */
const PHONE = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000a3");

/** The person the clients belong to. */
const DANA = schema.identifier("user").parse("user-019f5530-8000-7000-8000-0000000000a4");

test("key each destination by its channel and address, matching one address across its other fields", () => {
    // key one destination per channel, a browser by its client
    const browser = { channel: "push" as const, client: PHONE, user: DANA };
    const destinations: readonly Destination[] = [
        { channel: "desktop", client: LAPTOP },
        { channel: "email", address: "dana@example.com" },
        browser,
        { channel: "webhook" },
    ];

    // expect one key per destination, and the browser the same as itself alone
    expect([
        destinations.map((to) => Destination.key(to)),
        Destination.same(browser, { ...browser }),
        Destination.same(browser, { channel: "desktop", client: PHONE }),
    ]).toEqual([
        [`desktop:${LAPTOP}`, "email:dana@example.com", `push:${PHONE}`, "webhook"],
        true,
        false,
    ]);
});
