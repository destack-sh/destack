import { expect, test } from "@destack/test";
import { freePort, until } from "../server/test/fixture.ts";
import { TunnelClient } from "./client.ts";

test("refuse callers waiting for a tunnel once the client closes", async () => {
    // dial a relay that never answers, as a host with a token
    const client = TunnelClient.open({
        url: `http://127.0.0.1:${await freePort()}/tunnel`,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        retry: { initialInterval: 10, maximumInterval: 10 },
        report: () => {},
    });
    const waiting = Promise.allSettled([client.opened()]);

    // refuse the caller waiting before the close, and one asking after it
    await client.close();
    expect([await waiting, await Promise.allSettled([client.opened()])]).toEqual([
        [{ status: "rejected", reason: new Error("tunnel client is closed") }],
        [{ status: "rejected", reason: new Error("tunnel client is closed") }],
    ]);
});

test("keep dialing after the issuer grants no token, reporting each failed dial", async () => {
    // dial with a token the issuer refuses
    const reports: unknown[] = [];
    const url = `http://127.0.0.1:${await freePort()}/tunnel`;
    const client = TunnelClient.open({
        url,
        token: async () => {
            throw new TypeError("the issuer is unreachable");
        },
        fetch: async () => new Response(null, { status: 204 }),
        retry: { initialInterval: 10, maximumInterval: 10 },
        report: (error) => reports.push(error),
    });

    // report every failed dial and stop cleanly
    await until(() => reports.length >= 4);
    await client.close();
    expect(reports.slice(0, 2).map(String)).toEqual([
        "TypeError: the issuer is unreachable",
        `Error: tunnel to ${url} did not open`,
    ]);
});
