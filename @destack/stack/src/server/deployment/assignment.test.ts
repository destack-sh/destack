import { eq } from "@destack/db";
import { assignment } from "@destack/account/object";
import { TestDirectory } from "@destack/account/test";
import { Clock, schema, Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { build } from "../../object/index.ts";
import { AssignmentController } from "../index.ts";
import { ids, openUniverse, placeSpace, StackFixture } from "../../test/index.ts";

/** A laptop whose checkout a build of the space compiles. */
const LAPTOP = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000b1");

test("assign the machine of an unfinished checkout build in the space, and withdraw it once the build ends", async () => {
    // place the space on this machine with a pending build of the laptop's checkout and one of its own
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const opened = await openUniverse();
    await TestDirectory.place(opened.universe, {
        id: ids.space,
        scope: ids.account,
        machine: ids.machine,
        epoch: 1,
    });
    const now = Date.now();
    const record = async (checkoutMachine: typeof LAPTOP) => {
        const id = Identifier.create("build");
        await database.insert(build.table).values({
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
        });

        return id;
    };
    const laptop = await record(LAPTOP);
    await record(ids.machine);

    // assign only the laptop, reading the placement and assignments from the machine's replicas
    await placeSpace(database, ids.space, ids.machine);
    const publisher = await fixture.universe(opened.universe, ids.machine);
    const controller = new AssignmentController(database, publisher, ids.machine, Clock.system);
    const assignments = async () => {
        // replicate the universe's assignments to the machine, as its replica of them does
        const rows = await opened.universe.select().from(assignment.table);
        await database.delete(assignment.table);
        if (rows.length > 0) {
            await database.insert(assignment.table).values(rows);
        }

        return rows.map((row) => [row.placementId, row.machineId]);
    };
    expect(await controller.list()).toEqual([ids.space]);
    await controller.reconcile(ids.space);
    expect(await assignments()).toEqual([[ids.space, LAPTOP]]);

    // withdraw the laptop once its build is cancelled
    await database
        .update(build.table)
        .set({ state: "cancelled", finishedAt: now })
        .where(eq(build.table.id, laptop));
    await controller.reconcile(ids.space);
    expect(await assignments()).toEqual([]);
});
