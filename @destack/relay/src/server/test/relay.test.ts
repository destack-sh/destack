import { afterAll, beforeAll, expect, test } from "@destack/test";
import { directoryTables } from "@destack/directory";
import { account } from "@destack/account/object";
import { eq } from "@destack/db";
import { GlobalFixture, ids } from "@destack/host/test";
import { RequestId } from "@destack/service/request";
import { accountTables } from "@destack/account/stack";
import { defineDatabase } from "@destack/db/declare";
import { freePort, RelayFixture, until } from "./fixture.ts";

/** The global database of the hosts and accounts relays route to. */
const relayDatabase = defineDatabase({
    name: "global",
    tier: "global",
    tables: [...accountTables, ...directoryTables],
});

/** The global tier with each scenario's enrolled host. */
let global: GlobalFixture;

beforeAll(async () => {
    global = await GlobalFixture.open(relayDatabase);
});

afterAll(async () => {
    await global[Symbol.asyncDispose]();
});

test("forward a request for a space's name through its host's tunnel, keeping its name, method and body", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // reach the host's app by the space's name
    expect(
        await fixture.request(relay, fixture.notes, "/notes/1", { method: "POST", body: "pinned" }),
    ).toBe("hello POST /notes/1 pinned");
    expect([fixture.received, fixture.reports]).toEqual([[`POST ${fixture.notes}/notes/1`], []]);
});

test("reach a host by its name within its account through its tunnel", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // reach the host itself, and nothing by a name no standing host of the account has
    expect([
        await fixture.request(relay, await fixture.computer(), "/status"),
        await fixture.request(relay, "missing.acme.destack.computer", "/status"),
    ]).toEqual(["hello GET /status", "404 NOT_FOUND"]);
});

test("refuse names that lead nowhere, and a named host that keeps no tunnel", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());

    // refuse other handles, spaces, shapes and domains, and the host before it connects
    expect(
        await Promise.all(
            [
                `notes.${fixture.space}.rival.destack.space`,
                "notes.missing.acme.destack.space",
                `${fixture.space}.acme.destack.space`,
                `notes.${fixture.space}.acme.example.com`,
                fixture.notes,
            ].map((name) => fixture.request(relay, name, "/")),
        ),
    ).toEqual([
        "404 NOT_FOUND",
        "404 NOT_FOUND",
        "404 NOT_FOUND",
        "404 NOT_FOUND",
        "503 SERVICE_UNAVAILABLE",
    ]);
});

test("refuse a tunnel without a valid host token", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());

    const response = await fetch(relay.url, {
        headers: { upgrade: "websocket", connection: "upgrade", authorization: "Bearer forged" },
    });
    expect(response.status).toBe(401);
});

test("keep the route while the host renews its tunnel's token", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // outlive several renewals without a failure
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect([await fixture.request(relay, fixture.notes, "/"), fixture.reports]).toEqual([
        "hello GET /",
        [],
    ]);
});

test("restore the route when the host reconnects after its relay restarts", async () => {
    await using fixture = await RelayFixture.open(global);
    const port = await freePort();
    const first = fixture.relay(port);
    await fixture.tunnel(first);

    // restart the relay on the same port, and wait for the host to dial it again
    await fixture.stop(first);
    const second = fixture.relay(port);
    await until(
        async () => (await fixture.request(second, fixture.notes, "/again")) === "hello GET /again",
    );
});

test("route a branch to its space's host under the branch's name", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);

    const branch = `notes.feature-x--${fixture.space}.acme.destack.space`;
    expect([await fixture.request(relay, branch, "/"), fixture.received]).toEqual([
        "hello GET /",
        [`GET ${branch}/`],
    ]);
});

test("forward a name a region serves to the region's endpoint, keeping the host asked for", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    const cloud = `cloud-${fixture.space}`;
    await fixture.region("region");
    await fixture.place(cloud, ids.region);

    const name = `notes.${cloud}.acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`region host="${name}";proto=http`);
});

test("follow a space's move from its host to a region", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // move the zone to the region at the next epoch
    await fixture.region("region");
    await fixture.directory.place({
        id: fixture.spaceId!,
        scope: ids.account,
        cell: ids.region,
        epoch: 2,
    });

    await until(async () =>
        (await fixture.request(relay, fixture.notes, "/")).startsWith("region "),
    );
});

test("follow a space's rename", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // place the space under a new name, releasing the old one
    const renamed = `renamed-${fixture.space}`;
    await fixture.rename(renamed);

    // refuse the old name and reach the space by the new one
    await until(async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /");
    expect([
        await fixture.request(relay, fixture.notes, "/"),
        await fixture.request(relay, `notes.${renamed}.acme.destack.space`, "/"),
    ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
});

test("follow a handle's rename", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // rename the account's handle, and rename it back afterwards for the other scenarios
    const rename = (handle: string) =>
        global.database
            .update(account.table)
            .set({ handle })
            .where(eq(account.table.id, ids.account));
    await rename("acme-renamed");
    try {
        // refuse the old handle and reach the space by the new one
        await until(
            async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /",
        );
        expect([
            await fixture.request(relay, fixture.notes, "/"),
            await fixture.request(relay, fixture.notes.replace(".acme.", ".acme-renamed."), "/"),
        ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
    } finally {
        await rename("acme");
    }
});

test("follow a host's revocation", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    await fixture.tunnel(relay);
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the host as its owner
    await global.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: fixture.hostId,
        requestId: RequestId.create(),
    });

    await until(async () => (await fixture.request(relay, computer, "/")) === "404 NOT_FOUND");
});

test("follow a region's new endpoint", async () => {
    await using fixture = await RelayFixture.open(global);
    const relay = fixture.relay(await freePort());
    const cloud = `cloud-${fixture.space}`;
    await fixture.region("first");
    await fixture.place(cloud, ids.region);
    const name = `notes.${cloud}.acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`first host="${name}";proto=http`);

    // publish the region's new endpoint
    await fixture.region("second");

    await until(async () => (await fixture.request(relay, name, "/")).startsWith("second "));
});
