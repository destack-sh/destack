import { expect, test } from "@destack/test";
import { Authorization, Authorizer, principal } from "@destack/access";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import * as object from "../src/object/index.ts";
import { Capture } from "../src/object/index.ts";
import { eq } from "@destack/db";
import { Binder, resourceBindable, serveObjects } from "../src/server/index.ts";
import { ids, openSpace, spaceOptions } from "./fixture/space.ts";

test("capture what a deployment runs with: each binding at its target's generation, or at the version it pins", async () => {
    // keep two resources at different generations, bound by one installation, one binding pinned
    const database = await openSpace();
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const resource = (name: string, generation: number) => ({
        ...record,
        id: identifier("resource").parse(`resource-${v7()}`),
        name,
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "2026.9.0",
        definitionName: name,
        spec: {},
        generation,
    });
    const [main, archive, spare] = [
        resource("main", 3),
        resource("archive", 5),
        resource("spare", 1),
    ];
    await database.insert(object.resource.table).values([main, archive, spare]);
    await database.insert(object.installation.table).values({
        ...record,
        id: ids.notes,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "release", version: "2026.9.0" },
    } as never);
    const bind = (name: string, target: string, version: number | null) => ({
        ...record,
        id: identifier("binding").parse(`binding-${v7()}`),
        installationId: ids.notes,
        packageId: ids.package,
        name,
        target,
        version,
        state: {},
    });
    await database
        .insert(object.binding.table)
        .values([bind("main", main.id, null), bind("archive", archive.id, 4)]);

    // prepare a deployment of the installation's revision and capture its bindings
    const [revision] = await database
        .insert(object.installationRevision.table)
        .values({
            ...record,
            id: identifier("installation-revision").parse(`installation-revision-${v7()}`),
            installationId: ids.notes,
            build: { kind: "release", version: "2026.9.0" },
            views: {},
            settings: [],
            digest: "e".repeat(64),
        })
        .returning();
    const [created] = await database
        .insert(object.deployment.table)
        .values({
            ...record,
            id: identifier("deployment").parse(`deployment-${v7()}`),
            installationId: ids.notes,
            packageId: ids.package,
            revisionId: revision!.id,
            output: "main",
            workload: "main",
            runtime: "bun",
            release: "2026.9.0",
            status: "retired",
            description: {
                entrypoint: ".",
                services: [],
                triggers: [],
                resources: [],
                secrets: [],
                connections: [],
                compute: {},
            },
            policies: { packages: [], network: [] },
        })
        .returning();
    const server = serveObjects(await spaceOptions(database));
    const bindables = new Binder([resourceBindable]);
    const captured = await database.transaction((transaction) =>
        bindables.capture(
            {
                database: transaction,
                invoke: (invoked, name, input) =>
                    server.invoke(transaction, ids.space, invoked, name, input, Date.now()),
            },
            created!,
        ),
    );

    // run the unpinned binding at the target's generation and the pinned one at its pin
    expect(
        captured
            .map((entry) => [entry.name, entry.target, entry.version])
            .sort((left, right) => String(left[0]).localeCompare(String(right[0]))),
    ).toEqual([
        ["archive", archive.id, 4],
        ["main", main.id, 3],
    ]);

    // read the deployment's captures only while it is live
    const selector = {
        spaceId: ids.space,
        deploymentId: created!.id,
        installationId: ids.notes,
        packageId: ids.package,
        name: "main",
    };
    const before = await Capture.target(database, selector);
    await database
        .update(object.deployment.table)
        .set({ status: "active", activatedAt: Date.now() })
        .where(eq(object.deployment.table.id, created!.id));
    expect([before, await Capture.target(database, selector)]).toEqual([undefined, main.id]);

    // let the installation read exactly the resources its live deployment captured
    await bindables.relate(database, ids.space, ids.notes, Date.now());
    const workload = principal.installation.reference(ids.space, ids.notes);
    const authorization = new Authorization(
        new Authorizer(
            [object.space.policy, object.resource.policy],
            [object.space.mapping, object.resource.mapping],
        ),
        database,
        () => ({ subjects: [workload], now: Date.now(), attributes: {} }),
    );
    const reads = await Promise.all(
        [main, archive, spare].map((entry) =>
            authorization.check(
                object.resource.permission("read"),
                object.resource.reference(ids.space, entry.id),
            ),
        ),
    );
    expect(reads).toEqual([{ isAllowed: true }, { isAllowed: true }, { isAllowed: false }]);
});
