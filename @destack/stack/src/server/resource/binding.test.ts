import { expect, onTestFinished, test } from "@destack/test";
import { AccessContext, Authorization, Authorizer, Caller, principal } from "@destack/access";
import { Identifier, present } from "@destack/schema";
import * as object from "../../object/index.ts";
import * as accountObject from "@destack/account/object";
import { eq, databaseProvider, Transaction } from "@destack/db";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { serveSpace } from "../index.ts";
import { ids, reconcile, StackFixture } from "../../test/index.ts";

test("deploy an installation capturing each binding at its target's generation or its pinned version, and relate it to the captured resources", async () => {
    // keep two resources at different generations bound by one installation with one binding pinned
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const resource = (name: string, generation: number) => ({
        ...record,
        id: Identifier.create("database"),
        name,
        definitionPackageId: ids.package,
        definitionVersion: "2026.9.0",
        definitionName: name,
        spec: { replicated: [] },
        generation,
    });
    const [ledger, archive, spare] = [
        resource("ledger", 3),
        resource("archive", 5),
        resource("spare", 1),
    ];
    await database.insert(object.database.table).values([ledger, archive, spare]);
    await database.insert(object.installation.table).values({
        ...record,
        id: ids.notes,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
    });
    const bind = (name: string, target: string, version: number | null) => ({
        ...record,
        id: Identifier.create("binding"),
        installationId: ids.notes,
        packageId: ids.package,
        name,
        target,
        version,
        state: {},
    });
    await database
        .insert(object.binding.table)
        .values([bind("ledger", ledger.id, null), bind("archive", archive.id, 4)]);

    // follow the release and deploy it, capturing the bindings
    const [inserted] = await database
        .insert(object.installationRevision.table)
        .values({
            ...record,
            id: Identifier.create("installation-revision"),
            installationId: ids.notes,
            build: await fixture.release(ids.package, "2026.9.0"),
            views: {},
            settings: [],
            catalogs: [],
            digest: "e".repeat(64),
        })
        .returning();
    const revision = present(inserted, "the inserted revision");
    await database
        .update(object.installation.table)
        .set({ revisionId: revision.id })
        .where(eq(object.installation.table.id, ids.notes));
    const files = await mkdtemp(join(tmpdir(), "destack-stack-binding-"));
    onTestFinished(() => rm(files, { recursive: true, force: true }));
    const databases = new BunSqliteDatabaseHost(pathToFileURL(`${files}/`));
    const store = serveSpace(
        await fixture.options({ providers: [databaseProvider(databases, object.database)] }),
    ).store;
    await reconcile(store, "installation", { id: ids.notes });

    // run the unpinned binding at the target's generation and the pinned one at its pin
    const [deployed] = await database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.installationId, ids.notes));
    const created = present(deployed, "the deployment");
    const captured = await database
        .select()
        .from(object.capture.table)
        .where(eq(object.capture.table.deploymentId, created.id));
    expect(
        captured
            .map((entry) => [entry.name, entry.target, entry.version])
            .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
    ).toEqual([
        ["archive", archive.id, 4],
        ["ledger", ledger.id, 3],
    ]);

    // keep the deployment live
    expect(created.status).toBe("active");

    // let the installation read exactly the resources its live deployment captured
    const authorizer = new Authorizer(
        [accountObject.space.policy, object.database.policy],
        [accountObject.space.mapping, object.database.mapping],
    );
    const workload = principal.installation.reference(ids.space, ids.notes);
    const authorization = new Authorization(authorizer, Transaction.reading(database), () =>
        AccessContext.of(Caller.of(workload), Date.now()),
    );
    const reads = await Promise.all(
        [ledger, archive, spare].map((entry) =>
            authorization.check(
                object.database.permission("read"),
                object.database.reference(ids.space, entry.id),
            ),
        ),
    );
    expect(reads).toEqual([{ isAllowed: true }, { isAllowed: true }, { isAllowed: false }]);
});
