import { accessRelationship } from "@destack/access";
import { eq, Snapshot } from "@destack/db";
import { RequestId } from "@destack/runtime/request";
import { expect, test } from "@destack/test";
import { Account, placement, space } from "../src/object/index.ts";
import { AccountFixture, ids, outcome } from "../src/test/index.ts";

import { Claim } from "@destack/object";

test("create a space placed in the same transaction on its account's residency cloud machine or a named machine of the account or residency, and refuse a taken name leaving no placement", async () => {
    // let a person create spaces in their personal account, with a machine of their own enrolled in it
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const laptop = await fixture.enroll(owner.account, { client: owner.client });
    const outsider = await fixture.enroll(ids.account);
    const create = (name: string, machine?: typeof laptop.machineId) =>
        owner.client.space.create({
            accountId: owner.account,
            requestId: RequestId.create(Date.now()),
            name,
            ...(machine === undefined ? {} : { machine }),
        });

    // place one on the residency's cloud machine, one on the person's own machine and one on the named cloud machine
    const cloud = await create("work");
    const local = await create("notes", laptop.machineId);
    const shared = await create("shared", ids.machine);

    // refuse a taken name and a machine of another account
    const taken = await outcome(create("work"));
    const foreign = await outcome(create("other", outsider.machineId));
    expect({
        placements: await fixture.database
            .select({
                id: placement.table.id,
                scope: placement.table.scope,
                machine: placement.table.machineId,
                epoch: placement.table.epoch,
            })
            .from(placement.table)
            .where(eq(placement.table.scope, owner.account))
            .orderBy(placement.table.id),
        claimed: await Claim.lookup(
            space,
            {
                owner: async (index, key) =>
                    (await owner.client.claim.owner({ index, key })) ?? undefined,
            },
            "name",
            ["work"],
            owner.account,
        ),
        taken,
        foreign,
    }).toEqual({
        placements: [
            {
                id: Account.home(owner.account),
                scope: owner.account,
                machine: ids.machine,
                epoch: 1,
            },
            { id: cloud.id, scope: owner.account, machine: ids.machine, epoch: 1 },
            { id: local.id, scope: owner.account, machine: laptop.machineId, epoch: 1 },
            { id: shared.id, scope: owner.account, machine: ids.machine, epoch: 1 },
        ],
        claimed: { scope: owner.account, id: cloud.id },
        taken: "CONFLICT: name is taken",
        foreign: `BAD_REQUEST: ${outsider.machineId} is no standing machine of the space's account nor of its residency`,
    });
});

test("rename a space to a free address in its account, releasing the old one, and refuse a taken or malformed one", async () => {
    // let a person create two spaces in their personal account
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const create = (name: string) =>
        owner.client.space.create({
            accountId: owner.account,
            requestId: RequestId.create(Date.now()),
            name,
        });
    const first = await create("first");
    const second = await create("second");
    const rename = async (id: string, name: string) => {
        const renamed = owner.client.space.rename({
            accountId: owner.account,
            id,
            requestId: RequestId.create(Date.now()),
            name,
        });

        const settled = await outcome(renamed);

        return settled === "executed" ? (await renamed).name : settled;
    };

    // move the first space to a free address, refuse it to the second while taken, then release it
    const moved = await rename(first.id, "notes");
    const taken = await rename(second.id, "notes");
    const away = await rename(first.id, "personal");
    const released = await rename(second.id, "notes");

    // refuse malformed addresses
    const malformed = [
        await rename(second.id, "Notes"),
        await rename(second.id, "-notes"),
        await rename(second.id, "main--notes"),
    ];

    // refuse each by the address pattern
    const refused =
        "BAD_REQUEST: invalid input: name: labels are lowercase letters, digits and single inner hyphens";
    expect({ moved, taken, away, released, malformed }).toEqual({
        moved: "notes",
        taken: "CONFLICT: name is taken",
        away: "personal",
        released: "notes",
        malformed: [refused, refused, refused],
    });
});

test("create a space at the universe with its row alone, leaving its access to its home", async () => {
    // let a person create a space in their account
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const created = await owner.client.space.create({
        accountId: owner.account,
        requestId: RequestId.create(Date.now()),
        name: "work",
    });

    // store the row and its placement, and none of its relationships, at the universe
    const snapshot = Snapshot.live(fixture.database);
    expect({
        row: (await snapshot.read(space.table).findMany({ where: { id: created.id } })).map(
            (row) => row.name,
        ),
        placed: (await snapshot.read(placement.table).findMany({ where: { id: created.id } }))
            .length,
        tuples: await snapshot
            .read(accessRelationship)
            .findMany({ where: { objectId: created.id } }),
    }).toEqual({ row: ["work"], placed: 1, tuples: [] });
});
