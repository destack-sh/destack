import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { ObjectReference, Scope, type Subject } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";

import { ids, TestSpace } from "../../test/index.ts";
import stackPackage from "../../package.ts";
import { socket } from "../../object/index.ts";

/** The browser profile the owner's view runs on. */
const browser = schema.identifier("client").parse("client-01996ab0-0000-7000-8000-0000000000b1");

test("list a space's live sockets to its members only", async () => {
    // serve the owner's space with its sockets in memory
    const fixture = await TestSpace.open({});
    const sockets = await fixture.sockets();
    const served = await fixture.serve({ sockets });
    const listed = async (user: string | Subject) =>
        (await served.as(user).client(stackPackage).socket.list({ spaceId: ids.space })).items
            .toSorted((left, right) => left.kind.localeCompare(right.kind))
            .map((item) => [
                item.kind,
                ObjectReference.at("principal", item),
                item.clientId,
                item.session,
                item.installationId,
                item.lastActiveAt >= item.createdAt,
            ]);
    const machine = [
        "machine",
        principal.machine.reference(ids.account, ids.machine),
        null,
        null,
        null,
        true,
    ];

    // open a tab of the owner's notes on their browser and the machine connected to the space
    const owner = fixture.owner.subject;
    const tab = await sockets.connect(ids.space, {
        kind: "tab",
        principal: owner,
        client: browser,
        session: "client:5f1d",
        installation: ids.notes,
    });
    const linked = await sockets.connect(ids.space, {
        kind: "machine",
        principal: principal.machine.reference(ids.account, ids.machine),
    });

    // list both to the owner after the tab's heartbeat
    await tab.beat();
    expect(await listed(fixture.owner.id)).toEqual([
        machine,
        ["tab", owner, browser, "client:5f1d", ids.notes, true],
    ]);

    // drop the closed tab once and leave the machine
    await tab.close();
    await tab.close();
    expect(await listed(fixture.owner.id)).toEqual([machine]);
    await linked.close();
    expect(await listed(fixture.owner.id)).toEqual([]);

    // hide the space from a stranger, who cannot see it
    expect(await refusal(listed(principal.user.reference(Scope.universe.id, "stranger")))).toEqual([
        "NOT_FOUND",
        "SCOPE_NOT_FOUND",
        `scope ${ids.space} not found`,
    ]);
});

test("show a socket's client and session to its principal and the space's managers alone", async () => {
    // serve the owner's space, whose stack lets a viewer read its sockets
    const fixture = await TestSpace.open({});
    const sockets = await fixture.sockets();
    const served = await fixture.serve({ sockets });
    const owner = fixture.owner.subject;
    const person = await fixture.universe.signIn("viewer@example.com");
    const viewer = person.subject;
    await fixture.apply(served, [], {
        roles: {
            viewer: { description: "Sees who is here", permissions: [socket.permission("read")] },
        },
        relationships: { viewer: { role: "viewer", subject: { id: person.id } } },
    });

    // open a tab of the owner and one of the viewer
    const visitor = schema
        .identifier("client")
        .parse("client-01996ab0-0000-7000-8000-0000000000b2");
    await sockets.connect(ids.space, {
        kind: "tab",
        principal: owner,
        client: browser,
        session: "client:5f1d",
    });
    await fixture.simulation.clock.sleep(
        fixture.simulation.clock.now() + 1 - fixture.simulation.clock.now(),
    );
    await fixture.simulation.idle();
    await sockets.connect(ids.space, {
        kind: "tab",
        principal: viewer,
        client: visitor,
        session: "client:9c2e",
    });
    const listed = async (user: string | Subject) =>
        (await served.as(user).client(stackPackage).socket.list({ spaceId: ids.space })).items.map(
            (item) => [ObjectReference.at("principal", item), item.clientId, item.session],
        );

    // show the viewer its own tab's client and the owner every tab's
    expect({ viewer: await listed(person.id), owner: await listed(fixture.owner.id) }).toEqual({
        viewer: [
            [owner, undefined, undefined],
            [viewer, visitor, "client:9c2e"],
        ],
        owner: [
            [owner, browser, "client:5f1d"],
            [viewer, visitor, "client:9c2e"],
        ],
    });
});
