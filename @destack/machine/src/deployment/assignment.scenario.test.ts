import { eq } from "@destack/db";
import stackPackage from "@destack/stack/package";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { compile } from "@destack/stack/object";
import { assignment, placement } from "@destack/account/object";
import { Identifier, nth } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { fixtureSource, ids, TestSpace } from "../test/index.ts";

/** The origin the owner's laptop answers at. */
const LAPTOP_ORIGIN = "https://laptop.test";

test("assign the machine of an unfinished compile placed on it, and withdraw it once the build ends", async () => {
    // place the space on its machine with one compile on the owner's laptop and one on the space's runner
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.serve();
    const { id: laptopId } = await fixture.universe.enroll(
        fixture.owner,
        ids.account,
        LAPTOP_ORIGIN,
    );
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const record = async (runner: Identifier<"machine">, directory: string) => {
        // compile a directory of the fixture's commit, its step placed on the runner
        const created = await owner.build.create({
            spaceId: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            source: fixtureSource(directory),
            outputs: ["bun"],
        });
        const placed = await fixture.compileStep(created.id);
        const subject = principal.machine.reference(Scope.universe.id, runner);
        await database.transaction(
            async (transaction) =>
                await transaction
                    .update(compile.step.table)
                    .set(ObjectReference.columns("runner", subject))
                    .where(eq(compile.step.table.id, placed.id)),
        );

        return created.id;
    };
    const laptop = await record(laptopId, "laptop");
    await record(fixture.machine.id, ".");

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
