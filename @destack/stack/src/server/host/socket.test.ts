import { space } from "@destack/account/object";
import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, refusal, test } from "@destack/test";

import { ids, StackFixture } from "../../test/index.ts";

/** The browser profile the owner's view runs on. */
const browser = schema.identifier("client").parse("client-01996ab0-0000-7000-8000-0000000000b1");

test.for(TEST_DIALECTS)(
    "list a space's live sockets to its members only on %s",
    async (dialect) => {
        // serve the owner's space with its sockets in memory
        const fixture = await StackFixture.open({ dialect });
        const sockets = await fixture.sockets();
        const client = await fixture.serve({ sockets });
        const listed = async (user: string) =>
            (await client(user).socket.list({ spaceId: ids.space })).items
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
        const owner = principal.user.reference("universe", ids.owner);
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
        expect(await listed(ids.owner)).toEqual([
            machine,
            ["tab", owner, browser, "client:5f1d", ids.notes, true],
        ]);

        // drop the closed tab once and keep the machine
        await tab.close();
        await tab.close();
        expect(await listed(ids.owner)).toEqual([machine]);
        await linked.close();
        expect(await listed(ids.owner)).toEqual([]);

        // hide the space from a stranger, who cannot see it
        expect(await refusal(listed("stranger"))).toEqual(["NOT_FOUND", `no scope ${ids.space}`]);
    },
);

test.for(TEST_DIALECTS)(
    "show a socket's client and session to its principal and the space's managers alone on %s",
    async (dialect) => {
        // serve the owner's space with a viewer its home shares it with
        const fixture = await StackFixture.open({ dialect });
        const sockets = await fixture.sockets();
        const client = await fixture.serve({ sockets });
        const owner = principal.user.reference("universe", ids.owner);
        const viewer = principal.user.reference("universe", "viewer");
        await new AccessFixture(fixture.database).replicateRelationship(
            ids.account,
            space.reference(ids.account, ids.space),
            "viewer",
            viewer,
        );

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
        await sockets.connect(ids.space, {
            kind: "tab",
            principal: viewer,
            client: visitor,
            session: "client:9c2e",
        });
        const listed = async (user: string) =>
            (await client(user).socket.list({ spaceId: ids.space })).items
                .toSorted((left, right) => left.principalId.localeCompare(right.principalId))
                .map((item) => [
                    ObjectReference.at("principal", item),
                    item.clientId,
                    item.session,
                ]);

        // show the viewer its own tab's client and the owner every tab's
        expect({ viewer: await listed("viewer"), owner: await listed(ids.owner) }).toEqual({
            viewer: [
                [owner, undefined, undefined],
                [viewer, visitor, "client:9c2e"],
            ],
            owner: [
                [owner, browser, "client:5f1d"],
                [viewer, visitor, "client:9c2e"],
            ],
        });
    },
);
