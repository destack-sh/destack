import { eq } from "@destack/db";
import { assignment } from "@destack/account/object";
import { Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { build } from "../../object/index.ts";
import { AssignmentController } from "../index.ts";
import { ids, TestSpace } from "../../test/index.ts";

/** The origin the owner's laptop answers at. */
const LAPTOP_ORIGIN = "https://laptop.test";

test("assign the machine of an unfinished checkout build in the space, and withdraw it once the build ends", async () => {
    // place the space on its machine with a pending build of the owner's laptop's checkout and one of its own
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { id: laptopId } = await fixture.universe.enroll(
        fixture.owner,
        ids.account,
        LAPTOP_ORIGIN,
    );
    const now = fixture.machine.clock.now();
    const record = async (checkoutMachine: Identifier<"machine">) => {
        const id = Identifier.create("build", fixture.environment);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(build.table).values({
                    id,
                    scope: ids.space,
                    source: {
                        kind: "checkout",
                        machine: checkoutMachine,
                        checkout: ids.checkout,
                        directory: ".",
                    },
                    outputs: ["bun"],
                    createdAt: now,
                    updatedAt: now,
                }),
        );

        return id;
    };
    const laptop = await record(laptopId);
    await record(fixture.machine.id);

    // assign only the laptop through the machine's uplink, reading the assignments from the universe's rows
    const { publisher } = await fixture.options();
    const controller = new AssignmentController(
        database,
        publisher,
        fixture.machine.id,
        fixture.machine.environment,
    );
    const assignments = async () => {
        // replicate the universe's assignments to the machine, as its replica of them does
        const rows = await fixture.universe.storage.database.select().from(assignment.table);
        await database.transaction(
            async (transaction) => await transaction.delete(assignment.table),
        );
        if (rows.length > 0) {
            await database.transaction(
                async (transaction) => await transaction.insert(assignment.table).values(rows),
            );
        }

        return rows.map((row) => [row.placementId, row.machineId]);
    };
    expect(await controller.list()).toEqual([ids.space]);
    await controller.reconcile(ids.space);
    expect(await assignments()).toEqual([[ids.space, laptopId]]);

    // withdraw the laptop once its build is cancelled
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(build.table)
                .set({ state: "cancelled", finishedAt: now })
                .where(eq(build.table.id, laptop)),
    );
    await controller.reconcile(ids.space);
    expect(await assignments()).toEqual([]);
});
