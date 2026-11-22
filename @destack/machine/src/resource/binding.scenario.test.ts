import { installation } from "@destack/stack/object";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import type { BindingSelection } from "@destack/object";
import { AccessContext, Caller, principal, Authorization } from "@destack/access";
import { Authorizer } from "@destack/access/authorizer";
import { Identifier, present, schema } from "@destack/schema";
import * as object from "@destack/stack/object";
import * as accountObject from "@destack/account/object";
import { eq } from "@destack/db";
import { Transaction } from "@destack/db/database";
import { ids, insertInstallation, TestSpace } from "../test/index.ts";

test("capture each binding at its target's generation or pin, and relate its resources", async () => {
    // add two provisioned buckets at different generations, bound by one installation with one binding pinned
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const buckets = await fixture.buckets();
    const now = fixture.machine.clock.now();
    const record = {
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        ...ObjectReference.columns(
            "owner",
            principal.space.reference(Scope.universe.id, ids.space),
        ),
    };
    const resource = (name: string, generation: number) => ({
        ...record,
        id: Identifier.create("bucket", fixture.environment),
        name,
        definitionPackageId: ids.package,
        definitionVersion: "2026.9.1",
        definitionName: name,
        spec: {},
        generation,
        provider: buckets.provider,
        reference: name,
    });
    const [ledger, archive, spare] = [
        resource("ledger", 3),
        resource("archive", 5),
        resource("spare", 1),
    ];
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.bucket.table).values([ledger, archive, spare]),
    );
    await insertInstallation(
        database,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            follows: await fixture.followed(ids.package, "2026.9.1"),
            installer: fixture.owner.subject,
        },
        record.createdAt,
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({
                    bindings: {
                        [ids.package]: { ledger: bound(ledger.id), archive: bound(archive.id, 4) },
                    },
                })
                .where(eq(installation.table.id, ids.notes)),
    );

    // follow the release, bind its declared targets, and deploy it once the space serves it, capturing the bindings
    await fixture.release(ids.package, "2026.9.1");
    await fixture.simulation.idle();

    // run the unpinned binding at the target's generation and the pinned one at its pinned generation
    const [deployed] = await database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.installationId, ids.notes));
    const created = present(deployed, "the deployment");
    const [revision] = await database
        .select()
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, created.revisionId));
    const [target] = await database
        .select({ generation: object.bucket.table.generation })
        .from(object.bucket.table)
        .where(eq(object.bucket.table.id, ledger.id));
    expect(
        present(revision, "the deployment's revision")
            .bindings.map((entry) => [entry.name, entry.target, entry.version])
            .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
    ).toEqual([
        ["archive", archive.id, 4],
        ["ledger", ledger.id, present(target, "the ledger").generation],
    ]);

    // leave the deployment live
    expect(created.status).toBe("active");

    // let the installation read exactly the resources its live deployment captured
    const authorizer = new Authorizer(
        [accountObject.space.policy, object.bucket.policy],
        [accountObject.space.mapping, object.bucket.mapping],
    );
    const workload = principal.installation.reference(ids.space, ids.notes);
    const authorization = new Authorization(
        authorizer,
        Transaction.reading(database),
        () => AccessContext.of(Caller.of(workload), fixture.machine.clock.now()),
        fixture.environment,
    );
    const reads = await Promise.all(
        [ledger, archive, spare].map((entry) =>
            authorization.check(
                object.bucket.permission("read"),
                object.bucket.reference(ids.space, entry.id),
            ),
        ),
    );
    expect(reads).toEqual([{ isAllowed: true }, { isAllowed: true }, { isAllowed: false }]);
});

/** Bind a declaration to a bucket of the space, at a pinned version when given. */
function bound(target: string, version?: number): BindingSelection {
    return {
        target: { type: "bucket", id: schema.anyIdentifier().parse(target) },
        ...(version === undefined ? {} : { version }),
    };
}
