import accountPackage from "../src/package.ts";
import { accessRelationship, Role } from "@destack/access";
import { eq, Snapshot } from "@destack/db";
import { Claim, type ClaimLookup, type ClaimOwner } from "@destack/object";
import { RequestId } from "@destack/sync";
import { expect, test, refusal } from "@destack/test";
import { Account, placement, space } from "../src/object/index.ts";
import { TestAccountStore, ids, OWNER_EMAIL, type TestPerson } from "../src/test/index.ts";
import type { Identifier } from "@destack/schema";

/** Create a space of a name in a person's personal account, on a machine when given. */
function create(
    fixture: TestAccountStore,
    person: TestPerson,
    name: string,
    machine?: Identifier<"machine">,
) {
    return person.client(accountPackage).space.create({
        accountId: person.account,
        requestId: RequestId.create(fixture.environment),
        name,
        ...(machine === undefined ? {} : { machine }),
    });
}

/** Read the placements of a person's personal account, in the order of their spaces. */
function placements(fixture: TestAccountStore, person: TestPerson) {
    return fixture.database
        .select({
            id: placement.table.id,
            scope: placement.table.scope,
            machine: placement.table.machineId,
            epoch: placement.table.epoch,
        })
        .from(placement.table)
        .where(eq(placement.table.scope, person.account))
        .orderBy(placement.table.id);
}

test("place a new space on its account's residency cloud machine, on a machine of the account, or on a named cloud machine of the residency", async () => {
    // let a person create spaces in their personal account, with their machine enrolled in it
    await using fixture = await TestAccountStore.open();
    const person = await fixture.signIn("person@example.com");
    const laptop = await fixture.enroll(person.account, person);

    // place one on the residency's cloud machine, one on the person's machine and one on the named cloud machine, a millisecond apart so they order as created
    const cloud = await create(fixture, person, "work");
    fixture.clock.advance(1);
    const local = await create(fixture, person, "notes", laptop.machineId);
    fixture.clock.advance(1);
    const shared = await create(fixture, person, "shared", ids.machine);

    // store each placement beside the home's, and claim each name for its space
    const lookup: ClaimLookup = {
        resolve: async ({ index, keys }) => {
            // ask for each name's owner, keeping the claimed ones
            const owners = new Map<string, ClaimOwner>();
            for (const key of keys) {
                const owner = await person.client(accountPackage).claim.owner({ index, key });
                if (owner !== null) {
                    owners.set(key, owner);
                }
            }

            return owners;
        },
    };
    expect({
        placements: await placements(fixture, person),
        claimed: await Claim.lookup(space, lookup, "name", ["work"], person.account),
    }).toEqual({
        placements: [
            {
                id: Account.home(person.account),
                scope: person.account,
                machine: ids.machine,
                epoch: 1,
            },
            { id: cloud.id, scope: person.account, machine: ids.machine, epoch: 1 },
            { id: local.id, scope: person.account, machine: laptop.machineId, epoch: 1 },
            { id: shared.id, scope: person.account, machine: ids.machine, epoch: 1 },
        ],
        claimed: { scope: person.account, id: cloud.id },
    });
});

test("refuse a space under a taken name or on another account's machine, placing nothing", async () => {
    // let a person create a space, and enroll a machine of the owner's account
    await using fixture = await TestAccountStore.open();
    const person = await fixture.signIn("person@example.com");
    const owner = await fixture.signIn(OWNER_EMAIL);
    const elsewhere = await fixture.enroll(ids.account, owner);
    const work = await create(fixture, person, "work");

    // refuse the taken name and the other account's machine
    expect({
        taken: await refusal(create(fixture, person, "work")),
        foreign: await refusal(create(fixture, person, "other", elsewhere.machineId)),
        placements: (await placements(fixture, person)).map((placed) => placed.id),
    }).toEqual({
        taken: ["CONFLICT", "NAME_TAKEN", "name is taken"],
        foreign: [
            "BAD_REQUEST",
            "MACHINE_UNKNOWN",
            `${elsewhere.machineId} is not a standing machine of the space's account or residency`,
        ],
        placements: [Account.home(person.account), work.id],
    });
});

test("rename a space to a free address in its account, releasing the old one, and refuse a taken or malformed one", async () => {
    // let a person create two spaces in their personal account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const first = await create(fixture, owner, "first");
    const second = await create(fixture, owner, "second");
    const rename = async (id: string, name: string) => {
        const renamed = owner.client(accountPackage).space.rename({
            accountId: owner.account,
            id,
            requestId: RequestId.create(fixture.environment),
            name,
        });
        const settled = await refusal(renamed);

        return settled === "done" ? (await renamed).name : settled;
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
    const refused = [
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: name: labels are lowercase letters, digits and single inner hyphens",
    ];
    expect({ moved, taken, away, released, malformed }).toEqual({
        moved: "notes",
        taken: ["CONFLICT", "NAME_TAKEN", "name is taken"],
        away: "personal",
        released: "notes",
        malformed: [refused, refused, refused],
    });
});

test("create a space at the universe with its row and its own access, the space owning itself", async () => {
    // let a person create a space in their account at a clock standing still
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const created = await create(fixture, owner, "work");

    // store the row, its placement and the space's owner binding to itself at the universe
    const snapshot = Snapshot.live(fixture.database);
    const now = fixture.clock.now();
    expect({
        row: await fixture.database
            .select({ id: space.table.id, scope: space.table.scope, name: space.table.name })
            .from(space.table)
            .where(eq(space.table.id, created.id)),
        placed: await fixture.database
            .select({
                id: placement.table.id,
                scope: placement.table.scope,
                machineId: placement.table.machineId,
                epoch: placement.table.epoch,
                targetId: placement.table.targetId,
                transferId: placement.table.transferId,
                createdAt: placement.table.createdAt,
                updatedAt: placement.table.updatedAt,
            })
            .from(placement.table)
            .where(eq(placement.table.id, created.id)),
        tuples: (
            await snapshot.read(accessRelationship).findMany({ where: { objectId: created.id } })
        ).map((row) => [row.scope, row.roleId, row.subjectType, row.subjectId]),
    }).toEqual({
        row: [{ id: created.id, scope: owner.account, name: "work" }],
        placed: [
            {
                id: created.id,
                scope: owner.account,
                machineId: ids.machine,
                epoch: 1,
                targetId: null,
                transferId: null,
                createdAt: now,
                updatedAt: now,
            },
        ],
        tuples: [[owner.account, Role.OWNER, "space", created.id]],
    });
});
