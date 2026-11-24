import { eq } from "@destack/db";
import stackPackage from "@destack/stack/package";
import { RequestId } from "@destack/sync";
import { assignment, placement } from "@destack/account/object";
import { Identifier, nth } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { ids, TestSpace } from "../test/index.ts";

/** The origin the owner's laptop answers at. */
const LAPTOP_ORIGIN = "https://laptop.test";

test("assign the machine of an unfinished checkout build in the space, and withdraw it once the build ends", async () => {
    // place the space on its machine with a compiling build of the owner's laptop's checkout and one of its own
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.serve();
    const { id: laptopId } = await fixture.universe.enroll(
        fixture.owner,
        ids.account,
        LAPTOP_ORIGIN,
    );
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const record = async (checkoutMachine: Identifier<"machine">) => {
        const created = await owner.build.create({
            spaceId: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            source: {
                kind: "checkout",
                machine: checkoutMachine,
                checkout: ids.checkout,
                directory: ".",
            },
            outputs: ["bun"],
        });
        await fixture.compileStep(created.id);

        return created.id;
    };
    const laptop = await record(laptopId);
    await record(fixture.machine.id);

    // assign only the laptop through the machine's uplink, reading the assignments from the universe's rows
    const connections = new DisposableStack();
    onTestFinished(() => connections.dispose());
    const projection = (await fixture.space).assignment((await fixture.space).remote(connections));
    const reconciliation = {
        key: "key",
        signal: new AbortController().signal,
        changed: async () => {},
    };
    const assignments = async () => {
        // wait for the space's replica of the universe's assignments, which the fixture follows
        const rows = await fixture.universe.storage.database.select().from(assignment.table);
        const wanted = JSON.stringify(rows.map((row) => row.id).toSorted());
        const isReplicated = async () => {
            const replicated = await database
                .select({ id: assignment.table.id })
                .from(assignment.table);

            return JSON.stringify(replicated.map((row) => row.id).toSorted()) === wanted;
        };
        await database.log.waitUntil(isReplicated, new AbortController().signal);

        return rows.map((row) => [row.placementId, row.machineId]);
    };
    await projection.project(ids.space, reconciliation);
    const placed = await database
        .select({ id: placement.table.id })
        .from(placement.table)
        .where(eq(placement.table.spaceId, ids.space));
    expect(await assignments()).toEqual([[nth(placed, 0).id, laptopId]]);

    // withdraw the laptop once its build is cancelled, which ends its compile
    await owner.build.cancel({
        spaceId: ids.space,
        id: laptop,
        requestId: RequestId.create(fixture.machine.environment),
    });
    await fixture.finished(laptop);
    await projection.project(ids.space, reconciliation);
    expect(await assignments()).toEqual([]);
});
