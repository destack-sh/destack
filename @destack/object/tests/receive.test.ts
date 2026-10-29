import { AccessContext, accessRole, Authorization, Caller, principal } from "@destack/access";
import { asc, defineDatabase, Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { Installation, installation, PackageHandle, tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import objectServer from "../src/server/index.ts";
import { serversOf, testCallKey, testEvents, testSimulation } from "../src/test/index.ts";
import { fixturePackage } from "../src/test/fixture.ts";
import { notesPackage, note, notebook } from "./fixture/note.ts";
import { space } from "./fixture/space.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** A package whose name is longer than any installation's name. */
const longPackage = PackageHandle.of(
    Package.parse({
        id: "package-019f7480-0000-7000-8000-000000000002",
        name: "@destack/notification-observability",
        version: "2026.9.0",
    }),
    {},
);

test("receive a scope's base installations once each, unnamed whatever their package's name, and add a package's once the scope gains it", async () => {
    // store a space's object and notes packages in a migrated database
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({ packages: [objectPackage, notesPackage], follows: [], tables: [] }),
        }),
        { isMigrated: true, environment: world },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const scope = principal.space.reference(
        Scope.universe.id,
        "space-01996ab0-0000-7000-8000-000000000001",
    );
    const store = await TestObjectStore.at(scope, {
        chain: async () => undefined,
        environment: world,
        principal: scope,
        servers: [objectServer, ...serversOf([notebook, note])],
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: notesPackage },
    });

    // receive the space with the object package, then again with notes too
    const receive = (packages: Parameters<typeof Installation.receive>[3]) =>
        Transaction.run(database, async (transaction) => {
            const authorization = new Authorization(
                store.authorizer,
                transaction,
                () => AccessContext.of(Caller.of(scope), world.clock.now()),
                world,
            );
            await Installation.receive(authorization, scope, [], packages);
        });
    const rows = async () => ({
        installations: (
            await database
                .select({
                    packageId: installation.table.packageId,
                    name: installation.table.name,
                    selection: installation.table.selection,
                })
                .from(installation.table)
                .orderBy(asc(installation.table.packageId))
        ).map((row) => [row.packageId, row.name, row.selection]),
        roles: (
            await database
                .select({ name: accessRole.name })
                .from(accessRole)
                .orderBy(asc(accessRole.name))
        ).map((row) => row.name),
    });
    await receive([objectPackage]);
    const first = await rows();
    await receive([objectPackage, fixturePackage, longPackage]);
    const second = await rows();

    // keep the object package's base, and bear the others' unnamed once received with it
    expect({ first, second }).toEqual({
        first: {
            installations: [
                [objectPackage.id, null, { kind: "release", version: objectPackage.version }],
            ],
            roles: ["owner"],
        },
        second: {
            installations: [
                [fixturePackage.id, null, { kind: "release", version: fixturePackage.version }],
                [longPackage.id, null, { kind: "release", version: longPackage.version }],
                [objectPackage.id, null, { kind: "release", version: objectPackage.version }],
            ],
            // bind a base role only for a package requesting permissions in its scope, which none here does
            roles: ["owner"],
        },
    });
});
