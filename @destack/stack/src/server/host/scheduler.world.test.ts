import stackPackage from "../../package.ts";
import { Installation } from "@destack/object";
import { principal } from "@destack/access";
import { Tenancy, space } from "@destack/account/object";
import { eq } from "@destack/db";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { transfer } from "../../object/index.ts";
import accountPackage from "@destack/account/package";
import { ids, TestSpace } from "../../test/index.ts";

/** The endpoint the laptop's machine publishes. */
const LAPTOP_ENDPOINT = "https://laptop.test";

/** Open the space on its cloud machine next to the account's laptop, under a plan's tenancies. */
async function open(classes: readonly Tenancy[]) {
    // grant the account the plan's tenancies at the universe and refuse the others
    const fixture = await TestSpace.open();
    await fixture.universe.entitle(ids.account, "tenancy", { kind: "static", value: [...classes] });

    // serve the space on its residency's cloud machine next to the owner's laptop, placing it as each change settles
    const laptop = await fixture.universe.enroll(fixture.owner, ids.account, LAPTOP_ENDPOINT);
    const served = await fixture.serve();
    const settle = () => fixture.settle(served);
    await settle();

    return { fixture, served, laptop: laptop.id, settle };
}

/** Set the space's tenancy at the universe, as its owner does. */
async function choose(fixture: TestSpace, chosen: Tenancy | null): Promise<void> {
    const [row] = await fixture.database
        .select({ revision: space.table.revision })
        .from(space.table)
        .where(eq(space.table.id, ids.space));
    await fixture.owner.client(accountPackage).space.update({
        accountId: ids.account,
        id: ids.space,
        requestId: RequestId.create(fixture.environment),
        revision: present(row, "the space's row").revision,
        tenancy: chosen,
    });
}

/** Read the space's placement condition and the transfers started from the cloud machine. */
async function placement(fixture: TestSpace) {
    const [row] = await fixture.database
        .select({ conditions: space.table.conditions })
        .from(space.table)
        .where(eq(space.table.id, ids.space));
    const condition = row?.conditions["placement"];
    const transfers = (await fixture.database.select().from(transfer.table)).map((entry) => ({
        sourceId: entry.sourceId,
        targetId: entry.targetId,
        requestedBy: ObjectReference.at("requestedBy", entry),
        outcome: entry.outcome,
    }));

    return {
        condition: [condition?.status, condition?.reason, condition?.message],
        transfers,
    };
}

/** Read the space's base stack installation, which requests the space's moves. */
async function baseStack(fixture: TestSpace) {
    const scope = principal.space.reference(Scope.universe.id, ids.space);
    const { installation } = await Installation.base(
        scope,
        stackPackage,
        fixture.environment.crypto,
    );

    return principal.installation.reference(ids.space, installation);
}

test("keep a space where the universe decides it stays, and move it as its base stack once the universe decides a move", async () => {
    const { fixture, laptop, settle } = await open(["shared", "local"]);

    // keep the space on its machine while no tenancy is set
    const placed = await placement(fixture);

    // move it to the laptop once a person sets its tenancy to local
    await choose(fixture, "local");
    await settle();
    const moving = await placement(fixture);

    expect({ placed, moving }).toEqual({
        placed: {
            condition: ["true", "Unset", `placed shared on ${fixture.machine.id}`],
            transfers: [],
        },
        moving: {
            condition: ["unknown", "Moving", `moving to ${laptop}`],
            transfers: [
                {
                    sourceId: fixture.machine.id,
                    targetId: laptop,
                    requestedBy: await baseStack(fixture),
                    outcome: null,
                },
            ],
        },
    });
});

test("cancel a running move once the universe decides the space stays, and keep it on its machine", async () => {
    // start moving the space to the laptop, which never takes it over here
    const { fixture, laptop, settle } = await open(["shared", "local"]);
    await choose(fixture, "local");
    await settle();

    // unset its tenancy, which keeps it on its machine and cancels the move
    await choose(fixture, null);
    await settle();
    const kept = await placement(fixture);

    expect(kept).toEqual({
        condition: ["true", "Unset", `placed shared on ${fixture.machine.id}`],
        transfers: [
            {
                sourceId: fixture.machine.id,
                targetId: laptop,
                requestedBy: await baseStack(fixture),
                outcome: "cancelled",
            },
        ],
    });
});
