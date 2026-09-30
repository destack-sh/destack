import { Snapshot } from "@destack/db/log";
import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { type QueryPage, Scope } from "@destack/sync";
import { Condition } from "@destack/db/query";
import { TABLE } from "@destack/db";
import { zoneTable } from "@destack/directory";
import { zone } from "../src/object/index.ts";
import { v7 } from "uuid";
import { DirectoryClient } from "../src/client/index.ts";
import { Resolver } from "../src/directory/index.ts";
import { accountPackage } from "../src/audit/index.ts";
import { account } from "../src/object/index.ts";
import { AccountFixture, outcome, place, type Host } from "./fixture.ts";

test("place zones in the hosts creating them, keep names in them, let exactly their cell copy a space's access, refuse hosts exchanging tokens, and let each host copy the access of the account it serves", async () => {
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
    const zoneAt = (cell: string, epoch = 1) => ({ id: laptop, scope: accountId, cell, epoch });

    // create a zone as a host of the account and one as the region, and refuse the peer and the outsider
    expect([
        await outcome(directory(local).place(zoneAt(local.id))),
        await outcome(directory(peer).place(zoneAt(peer.id))),
        await outcome(directory(outsider).place({ ...zoneAt(outsider.id), id: foreign })),
        await outcome(directory(regional).place({ ...zoneAt(regionId), id: cloud })),
    ]).toEqual([
        "executed",
        `FORBIDDEN: ${laptop} is served by another host or region`,
        `FORBIDDEN: this host creates no zone ${foreign} in ${accountId} for ${outsider.id}`,
        "executed",
    ]);
    expect(await directory(peer).locate(laptop)).toEqual(zoneAt(local.id));

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

    // let exactly the cell of each place copy the chain above it, and each host the account it serves
    const copies = async (host: Host, spaceId?: typeof laptop, scope: string = accountId) => {
        const controller = new AbortController();
        try {
            const pages = await host.client.replica.stream(
                {
                    name: "chain",
                    scope,
                    below: spaceId ?? scope,
                    access: true,
                    held: [],
                    copied: [],
                    rows: [],
                },
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

    // carry the account's own row in the chain copy of a cell keeping accounts, with the fields guarded from the space concealed
    const following = new AbortController();
    const chain = await local.client.replica.stream(
        {
            name: "chain",
            scope: accountId,
            below: laptop,
            access: true,
            held: [],
            copied: [
                {
                    packageId: account.policy.definition.packageId,
                    type: account.policy.definition.name,
                },
            ],
            rows: [],
        },
        { signal: following.signal },
    );
    const first = (await (chain as AsyncIterable<QueryPage>)[Symbol.asyncIterator]().next())
        .value as QueryPage;
    following.abort();
    expect(
        first.changes
            .filter((change) => change.table === "destack__account__account")
            .map((change) => [change.row.id, change.concealed]),
    ).toEqual([
        [
            accountId,
            [
                "defaultResidency",
                "packagePolicyId",
                "packagePolicyRegion",
                "networkPolicyId",
                "networkPolicyRegion",
            ],
        ],
    ]);

    // refuse exchanging tokens for hosts, which the host service grants
    expect(
        await outcome(
            local.client.authentication.exchange({ audience: accountPackage.id, spaceId: laptop }),
        ),
    ).toBe("UNAUTHORIZED: the caller is not an account caller");

    // place the laptop place in the region at the next epoch, after which only the region copies its access
    await directory(local).place(zoneAt(regionId, 2));
    expect([
        await outcome(directory(local).place(zoneAt(local.id, 3))),
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
        { ...zoneAt(regionId, 2), endpoint: "https://region.test/" },
        `NOT_FOUND: nothing answers at missing.${handle}`,
    ]);

    // move and withdraw only the zones a host's cells serve, and let a cell copy the zones moving to it alone
    const zones = async (host: Host, cell: string) => {
        const controller = new AbortController();
        try {
            const pages = await host.client.replica.stream(
                {
                    name: cell,
                    scope: Scope.universe.id,
                    below: cell,
                    access: false,
                    held: [],
                    copied: [],
                    rows: [
                        {
                            type: {
                                packageId: zone.policy.definition.packageId,
                                type: zone.policy.definition.name,
                            },
                            where: Condition.all(),
                        },
                    ],
                },
                { signal: controller.signal },
            );
            const { value } = await pages[Symbol.asyncIterator]().next();

            return (value as QueryPage).changes
                .filter((change) => change.table === zoneTable[TABLE].sqlName)
                .map((change) => [change.row.id, change.row.target]);
        } catch (error) {
            return (error as { code: string }).code;
        } finally {
            controller.abort();
        }
    };
    expect([
        await outcome(directory(local).move(zoneAt(regionId, 2), local.id)),
        await outcome(directory(regional).move(zoneAt(regionId, 2), local.id)),
        await zones(local, local.id),
        await zones(peer, peer.id),
        await zones(local, regionId),
        await outcome(directory(peer).withdraw(zoneAt(regionId, 2))),
        await outcome(directory(regional).withdraw(zoneAt(regionId, 2))),
    ]).toEqual([
        `FORBIDDEN: ${laptop} is served by another host or region`,
        "executed",
        [[laptop, local.id]],
        [],
        "FORBIDDEN",
        `FORBIDDEN: ${laptop} is served by another host or region`,
        "executed",
    ]);
    expect(await directory(local).locate(laptop)).toBeUndefined();
});
