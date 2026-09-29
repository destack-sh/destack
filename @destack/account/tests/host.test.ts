import { Snapshot } from "@destack/db/log";
import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import type { QueryPage } from "@destack/sync";
import { v7 } from "uuid";
import { DirectoryClient } from "../src/client/index.ts";
import { Resolver } from "../src/directory/index.ts";
import { accountPackage } from "../src/audit/index.ts";
import { AccountFixture, outcome, place, type Host } from "./fixture.ts";

test("place zones in the hosts creating them, keep names in them, let exactly their cell copy a space's access and exchange its tokens, and let each host copy the access of the account it serves", async () => {
    // enroll two hosts of the owner's account, one of another account, and one serving a region
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, handle } = await fixture.createSpace(owner);
    const other = await fixture.createSpace(owner);
    const regionId = identifier("region").parse(`region-${v7()}`);
    const [local, peer, outsider, regional] = [
        fixture.host(accountId),
        fixture.host(accountId),
        fixture.host(other.accountId),
        fixture.host(other.accountId, regionId),
    ];
    const directory = (host: Host) => new DirectoryClient(host.client);
    const resolver = (host: Host) => {
        const client = directory(host);

        return new Resolver(client, (named) => client.account(named));
    };
    const laptop = identifier("space").parse(`space-${v7()}`);
    const cloud = identifier("space").parse(`space-${v7()}`);
    const foreign = identifier("space").parse(`space-${v7()}`);
    const zone = (cell: string, epoch = 1) => ({ id: laptop, scope: accountId, cell, epoch });

    // create a zone as a host of the account and one as the region, and refuse the peer and the outsider
    expect([
        await outcome(directory(local).place(zone(local.id))),
        await outcome(directory(peer).place(zone(peer.id))),
        await outcome(directory(outsider).place({ ...zone(outsider.id), id: foreign })),
        await outcome(directory(regional).place({ ...zone(regionId), id: cloud })),
    ]).toEqual([
        "executed",
        `FORBIDDEN: ${laptop} is served by another host or region`,
        `FORBIDDEN: this host creates no zone ${foreign} in ${accountId} for ${outsider.id}`,
        "executed",
    ]);
    expect(await directory(peer).locate(laptop)).toEqual(zone(local.id));

    // reserve and confirm the laptop place's name as its cell and refuse the peer
    const row = { id: laptop, scope: accountId, name: "notes" };
    const snapshot = Snapshot.live(fixture.opened.database);
    const claims = await place.claims(row, snapshot);
    const requestId = RequestId.create();
    expect(await outcome(directory(peer).claim(claims, requestId))).toBe(
        "FORBIDDEN: claims lie outside the zones this host serves",
    );
    await directory(local).claim(claims, requestId);
    await directory(local).confirm(requestId, [await place.owned(laptop, row, snapshot)]);
    expect(await place.lookup(directory(outsider), "name", ["notes"], accountId)).toEqual(
        place.reference(accountId, laptop),
    );

    // let exactly the cell of each place copy the access above it, and each host the account it serves
    const copies = async (host: Host, spaceId?: typeof laptop, scope: string = accountId) => {
        const controller = new AbortController();
        try {
            const pages = await host.client.access.watch(
                { ...(spaceId === undefined ? {} : { spaceId }), scope, held: [], copied: [] },
                { signal: controller.signal },
            );
            await (pages as AsyncIterable<QueryPage>)[Symbol.asyncIterator]().next();

            return "copied";
        } catch (error) {
            return (error as { code: string }).code;
        } finally {
            controller.abort();
        }
    };
    expect([
        [await copies(local, laptop), await copies(local, cloud)],
        [await copies(peer, laptop), await copies(regional, cloud)],
        [await copies(peer), await copies(peer, undefined, other.accountId)],
        [await copies(outsider, undefined, other.accountId), await copies(outsider)],
    ]).toEqual([
        ["copied", "FORBIDDEN"],
        ["FORBIDDEN", "copied"],
        ["copied", "FORBIDDEN"],
        ["copied", "FORBIDDEN"],
    ]);

    // stream the members' public profiles and handles to the cell alone: nothing but name, image and handle
    const profiles = async (host: Host) => {
        const controller = new AbortController();
        try {
            const pages = await host.client.access.profiles(
                { spaceId: laptop, users: [owner.id] },
                { signal: controller.signal },
            );
            const { value } = await pages[Symbol.asyncIterator]().next();

            return value;
        } catch (error) {
            return (error as { code: string }).code;
        } finally {
            controller.abort();
        }
    };
    const streamed = [await profiles(local), await profiles(peer)];
    expect(streamed).toEqual([
        {
            reset: true,
            complete: true,
            position: await fixture.opened.database.log.position(),
            profiles: [{ id: owner.id, name: "owner@example.com", image: null }],
            removed: [],
            handles: [{ accountId, userId: owner.id, handle }],
            unhandled: [],
        },
        "FORBIDDEN",
    ]);

    // issue tokens of the laptop place to its cell alone, acting as no one but itself
    const exchange = (host: Host, subject?: typeof owner.subject) =>
        host.client.authentication.exchange({
            audience: accountPackage.id,
            spaceId: laptop,
            ...(subject === undefined ? {} : { subject }),
        });
    expect([
        (await exchange(local)).tokenType,
        await outcome(exchange(peer)),
        await outcome(exchange(local, owner.subject)),
    ]).toEqual([
        "Bearer",
        `FORBIDDEN: ${laptop} is served elsewhere`,
        "FORBIDDEN: a host acts as no one but itself",
    ]);

    // place the laptop place in the region at the next epoch, after which only the region copies its access
    await directory(local).place(zone(regionId, 2));
    expect([
        await outcome(directory(local).place(zone(local.id, 3))),
        await copies(local, laptop),
        await copies(regional, laptop),
    ]).toEqual([`FORBIDDEN: ${laptop} is served by another host or region`, "FORBIDDEN", "copied"]);

    // publish endpoints only for a host's own cells in their scopes, and read them for anyone
    expect([
        await outcome(
            local.client.directory.publish({
                cell: local.id,
                scope: accountId,
                endpoint: "https://local.test/",
            }),
        ),
        await outcome(
            local.client.directory.publish({
                cell: local.id,
                scope: other.accountId,
                endpoint: "https://local.test/",
            }),
        ),
        await outcome(
            local.client.directory.publish({
                cell: regionId,
                scope: "universe",
                endpoint: "https://region.test/",
            }),
        ),
        await outcome(
            regional.client.directory.publish({
                cell: regionId,
                scope: "universe",
                endpoint: "https://region.test/",
            }),
        ),
    ]).toEqual([
        "executed",
        `FORBIDDEN: ${local.id} belongs to another scope than ${other.accountId}`,
        `FORBIDDEN: this host acts for no cell ${regionId}`,
        "executed",
    ]);
    expect([
        await directory(outsider).cell(local.id),
        await directory(outsider).cell(regionId),
        await resolver(outsider).resolve(place, `notes.${handle}`),
        await outcome(resolver(outsider).resolve(place, `missing.${handle}`)),
    ]).toEqual([
        { id: local.id, scope: accountId, endpoint: "https://local.test/" },
        { id: regionId, scope: "universe", endpoint: "https://region.test/" },
        { ...zone(regionId, 2), endpoint: "https://region.test/" },
        `NOT_FOUND: nothing answers at missing.${handle}`,
    ]);

    // move, follow and withdraw only the zones a host's cells serve or receive
    expect([
        await outcome(directory(local).move(zone(regionId, 2), local.id)),
        await outcome(directory(regional).move(zone(regionId, 2), local.id)),
        await outcome(
            directory(local)
                .incoming(regionId, AbortSignal.timeout(1000))
                [Symbol.asyncIterator]()
                .next(),
        ),
        await outcome(directory(peer).withdraw(zone(regionId, 2))),
        await outcome(directory(regional).withdraw(zone(regionId, 2))),
    ]).toEqual([
        `FORBIDDEN: ${laptop} is served by another host or region`,
        "executed",
        `FORBIDDEN: this host acts for no cell ${regionId}`,
        `FORBIDDEN: ${laptop} is served by another host or region`,
        "executed",
    ]);
    expect(await directory(local).locate(laptop)).toBeUndefined();
});
