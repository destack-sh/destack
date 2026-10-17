import { TestEntitlement, TestDirectory } from "@destack/account/test";
import { principal } from "@destack/access";
import { Tenancy, machine, space } from "@destack/account/object";
import { eq } from "@destack/db";
import { reconciliation } from "@destack/runtime/test";
import { setting } from "@destack/setting/object";
import { schema, present } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { transfer } from "../../object/index.ts";
import { tenancy } from "../../setting/index.ts";
import { ids, openUniverse, StackFixture } from "../../test/index.ts";
import { serveSpace } from "../server.ts";

/** The account's personal machine, which runs its spaces of the own tenancy. */
const LAPTOP = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000041");

/** The account's own cloud machine, which runs its spaces of dedicated tenancy. */
const RESERVED = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000043");

/** The endpoint the laptop's machine publishes. */
const LAPTOP_ENDPOINT = "https://laptop.test";

/** The endpoint the account's own cloud machine publishes. */
const DEDICATED_ENDPOINT = "https://dedicated.test";

/** Open the cloud machine serving the fixture's space beside the account's laptop. */
async function open(classes: readonly Tenancy[]) {
    // place the space in the cloud machine and publish the laptop's machine in the directory
    const fixture = await StackFixture.open();
    const opened = await openUniverse();
    await TestDirectory.place(opened.universe, {
        id: ids.space,
        scope: ids.account,
        machine: ids.cloud,
        epoch: 1,
    });
    await TestDirectory.publish(opened.universe, LAPTOP, ids.account, LAPTOP_ENDPOINT);

    // replicate the account's laptop as the cloud machine replicates its machines
    const now = Date.now();
    await fixture.database.insert(machine.table).values({
        id: LAPTOP,
        scope: ids.account,
        name: "laptop",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
        createdAt: now,
        updatedAt: now,
    });

    // grant the account the plan's tenancies and refuse the others
    await TestEntitlement.set(fixture.database, ids.account, "tenancy", {
        kind: "static",
        value: [...classes],
    });

    // reconcile each space as the space controller does after a change
    const objects = serveSpace(
        await fixture.options({
            ...opened,
            isEnforcingEntitlements: true,
            publisherAt: () => TestEntitlement.publisher,
        }),
    ).store;
    const controller = present(
        objects.controllers().find((each) => each.name === "space"),
        "the space controller",
    );
    const control = async () => {
        for (const key of await controller.list()) {
            await controller.reconcile(key, reconciliation());
        }
    };

    return { fixture, universe: opened.universe, control };
}

/** Set the space's tenancy as a person does. */
async function choose(fixture: StackFixture, chosen: Tenancy): Promise<void> {
    const now = Date.now();
    await fixture.database
        .insert(setting.table)
        .values({
            id: setting.identifier("setting-01996ab0-0000-7000-8000-000000000042"),
            scope: ids.space,
            createdAt: now,
            updatedAt: now,
            packageId: tenancy.reference.packageId,
            name: tenancy.reference.name,
            mode: "set",
            value: chosen,
            release: tenancy.package.version,
        })
        .onConflictDoUpdate({ target: setting.table.id, set: { value: chosen } });
}

/** Read the space's placement condition and the transfers started from the cloud machine. */
async function placement(fixture: StackFixture) {
    const [row] = await fixture.database
        .select({ conditions: space.table.conditions })
        .from(space.table)
        .where(eq(space.table.id, ids.space));
    const condition = row?.conditions["placement"];
    const transfers = (await fixture.database.select().from(transfer.table)).map((entry) => ({
        sourceId: entry.sourceId,
        targetId: entry.targetId,
        requestedBy: ObjectReference.at("requestedBy", entry),
    }));

    return {
        condition: [condition?.status, condition?.reason, condition?.message],
        transfers,
    };
}

test("place a shared space on its residency's cloud machine and move it to the account's own machine", async () => {
    const { fixture, control } = await open(["shared", "own"]);

    // place the space where the default tenancy asks
    await control();
    const placed = await placement(fixture);

    // move it to the laptop once a person sets its tenancy to own
    await choose(fixture, "own");
    await control();
    await control();
    const moving = await placement(fixture);

    expect({ placed, moving }).toEqual({
        placed: { condition: ["true", "Placed", `placed shared on ${ids.cloud}`], transfers: [] },
        moving: {
            condition: ["unknown", "Moving", `moving to ${LAPTOP}`],
            transfers: [
                {
                    sourceId: ids.cloud,
                    targetId: LAPTOP,
                    requestedBy: principal.space.reference(Scope.universe.id, ids.space),
                },
            ],
        },
    });
});

test("keep a space pending until its machine's own row is replicated, then place it", async () => {
    const { fixture, control } = await open(["shared", "own"]);

    // drop the cloud machine's replicated row, as before its directory replica brings it
    const [kept] = await fixture.database
        .select()
        .from(machine.table)
        .where(eq(machine.table.id, ids.cloud));
    await fixture.database.delete(machine.table).where(eq(machine.table.id, ids.cloud));
    await control();
    const pending = await placement(fixture);

    // place the space once the row arrives
    await fixture.database.insert(machine.table).values(present(kept, "the cloud machine's row"));
    await control();
    const placed = await placement(fixture);

    expect({ pending, placed }).toEqual({
        pending: {
            condition: ["unknown", "Pending", `waiting for the replica of machine ${ids.cloud}`],
            transfers: [],
        },
        placed: { condition: ["true", "Placed", `placed shared on ${ids.cloud}`], transfers: [] },
    });
});

test("refuse dedicated tenancy the plan does not unlock, and move the space to the account's own cloud machine once the plan unlocks it", async () => {
    // refuse dedicated under a plan unlocking only shared and own
    const limited = await open(["shared", "own"]);
    await choose(limited.fixture, "dedicated");
    await limited.control();
    const unentitled = await placement(limited.fixture);

    // replicate and publish the account's own cloud machine beside its laptop under a plan unlocking every tenancy
    const unlocked = await open(["shared", "dedicated", "own"]);
    await TestDirectory.publish(unlocked.universe, RESERVED, ids.account, DEDICATED_ENDPOINT);
    const now = Date.now();
    await unlocked.fixture.database.insert(machine.table).values({
        id: RESERVED,
        scope: ids.account,
        name: "dedicated",
        kind: "cloud",
        residencyId: "eu",
        providerCode: "cloudflare",
        location: "eu",
        createdAt: now,
        updatedAt: now,
    });

    // move the space there once a person sets its tenancy to dedicated
    await choose(unlocked.fixture, "dedicated");
    await unlocked.control();
    const moving = await placement(unlocked.fixture);

    expect({ unentitled, moving }).toEqual({
        unentitled: {
            condition: ["false", "Unentitled", "no plan of the account grants dedicated tenancy"],
            transfers: [],
        },
        moving: {
            condition: ["unknown", "Moving", `moving to ${RESERVED}`],
            transfers: [
                {
                    sourceId: ids.cloud,
                    targetId: RESERVED,
                    requestedBy: principal.space.reference(Scope.universe.id, ids.space),
                },
            ],
        },
    });
});
