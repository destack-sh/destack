import { accessRelationship, principal } from "@destack/access";
import { space } from "@destack/account/object";
import { databaseProvider, eq, TABLE } from "@destack/db";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId, replica, Scope } from "@destack/sync";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    build,
    database,
    installation,
    installationRevision,
    MAIN_DATABASE,
} from "../src/object/index.ts";
import { PlacementController, serveSpace } from "../src/server/index.ts";
import stackPackage from "../src/package.ts";
import {
    replicateAccount,
    ids,
    openUniverse,
    placeSpace,
    spaceDatabase,
    StackFixture,
    TestSpaceDatabaseHost,
} from "../src/test/index.ts";

/** The stack package the spaces install. */
const stacked = Package.parse({ id: ids.stackPackage, name: "@test/stack", version: "2026.9.0" });

test("give birth to placed spaces from their replicated rows with main in their own storage, refuse a stack declaring main, and resume a stack left unsubmitted", async () => {
    // publish a stack without main and one declaring it
    const fixture = await StackFixture.empty();
    await replicateAccount(fixture.database);
    await replicateChain(fixture);
    const main = {
        declaration: spaceDatabase.toJSON(),
        placement: { provider: "postgresql" },
        retention: "forever",
        tags: {},
    };
    const stackBuild = await MemoryBuild.of(stacked, [
        {
            kind: "stack",
            kindPackageId: stackPackage.id,
            name: "home",
            description: { resources: {}, installations: {} },
        },
        {
            kind: "stack",
            kindPackageId: stackPackage.id,
            name: "placed",
            description: { resources: { main }, installations: {} },
        },
    ]);
    const manifest = await fixture.registry.push(stackBuild);

    // place two spaces carrying their stacks, with the rows the machine replicates
    const home = schema.identifier("space").parse(Identifier.create("space"));
    const placed = schema.identifier("space").parse(Identifier.create("space"));
    const now = Date.now();
    const row = (id: Identifier<"space">, name: string) => ({
        id,
        scope: ids.account,
        name,
        stack: { package: stacked, manifest, export: name, settings: {} },
        createdAt: now,
        updatedAt: now,
    });
    const opened = await openUniverse();
    await fixture.database.insert(space.table).values([row(home, "home"), row(placed, "placed")]);
    for (const spaceId of [home, placed]) {
        await placeSpace(fixture.database, spaceId);
    }

    // serve the machine giving birth to the spaces
    const directory = await mkdtemp(join(tmpdir(), "destack-placement-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const host = new TestSpaceDatabaseHost(pathToFileURL(`${directory}/`));
    const place = async (spaceId: Identifier<"space">) => {
        // receive the space through its own store, as each space's workload does
        const options = await fixture.options({
            ...opened,
            principal: principal.space.reference(Scope.universe.id, spaceId),
            providers: [databaseProvider(host, database)],
        });
        const { store } = serveSpace(options);
        const controller = new PlacementController(store, ids.cloud, options);

        return controller.reconcile(spaceId);
    };

    // give birth to the spaces, refusing the stack declaring main, then forget the home's submission as a crash before it would
    await place(home);
    const refused = await refusal(place(placed));
    await fixture.database
        .update(installation.table)
        .set({ revisionId: null, spaceStack: null })
        .where(eq(installation.table.scope, home));
    await place(home);

    // read each space's main placement and its stack installation
    const mains = await fixture.database
        .select({ scope: database.table.scope, placement: database.table.placement })
        .from(database.table)
        .where(eq(database.table.name, MAIN_DATABASE));
    const stacks = await fixture.database
        .select({ scope: installation.table.scope, revisionId: installation.table.revisionId })
        .from(installation.table)
        .where(eq(installation.table.role, "stack"));

    expect({
        mains: mains.map((entry) => entry.placement),
        stacks: stacks.map((entry) => [entry.scope, entry.revisionId !== null]),
        refused,
    }).toEqual({
        // keep main in each space's own storage
        mains: [{ provider: "space" }, { provider: "space" }],
        // install the home's stack once, submitting it again after the crash, and refuse the other's
        stacks: [[home, true]],
        refused: ["INVALID_DEFINITION", "stack placed declares main, the database every space has"],
    });
});

test("keep a person's own apply of a placed space's stack, and submit the space's stack again once it changes, once only", async () => {
    // publish two builds of the stack
    const fixture = await StackFixture.empty();
    await replicateAccount(fixture.database);
    await replicateChain(fixture);
    const declaring = (title: string) =>
        MemoryBuild.of(stacked, [
            {
                kind: "stack",
                kindPackageId: stackPackage.id,
                name: "home",
                description: { resources: {}, installations: {}, title },
            },
        ]);
    const first = await fixture.registry.push(await declaring("first"));
    const second = await fixture.registry.push(await declaring("second"));

    // place a home carrying the first build
    const home = schema.identifier("space").parse(Identifier.create("space"));
    const now = Date.now();
    const carried = { package: stacked, manifest: first, export: "home", settings: {} };
    await fixture.database.insert(space.table).values({
        id: home,
        scope: ids.account,
        name: "home",
        stack: carried,
        createdAt: now,
        updatedAt: now,
    });
    await placeSpace(fixture.database, home);

    // serve the machine giving birth to the home through its own store
    const opened = await openUniverse();
    const directory = await mkdtemp(join(tmpdir(), "destack-placement-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const host = new TestSpaceDatabaseHost(pathToFileURL(`${directory}/`));
    const options = await fixture.options({
        ...opened,
        principal: principal.space.reference(Scope.universe.id, home),
        providers: [databaseProvider(host, database)],
    });
    const { store } = serveSpace(options);
    const controller = new PlacementController(store, ids.cloud, options);

    // give birth to the home
    await controller.reconcile(home);

    // read the home's stack installation and the revision it follows
    const stack = async () => {
        const [row] = await fixture.database
            .select({ installed: installation.table, revision: installationRevision.table })
            .from(installation.table)
            .innerJoin(
                installationRevision.table,
                eq(installationRevision.table.id, installation.table.revisionId),
            )
            .where(eq(installation.table.scope, home));

        return present(row, "the home's stack");
    };

    // read the manifest of a pulled release build, or mark a compiled one
    const pulledManifest = async (buildId: Identifier<"build">) => {
        const [row] = await fixture.database
            .select({ source: build.table.source, manifest: build.table.manifest })
            .from(build.table)
            .where(eq(build.table.id, buildId));
        const found = present(row, `build ${buildId}`);

        return found.source.kind === "release" ? found.manifest : "build";
    };

    // read the build and settings the home's stack follows after each reconcile
    const color = `${ids.stackPackage}/color`;
    const followed = async () => {
        await controller.reconcile(home);
        const { revision } = await stack();

        return {
            manifest: await pulledManifest(revision.buildId),
            settings: present(revision.definition, "the definition")["settings"],
            listed: await controller.list(),
        };
    };

    // apply the second build as the person does, without the space's stack
    const apply = async () => {
        const { installed, revision } = await stack();
        const evaluation = { export: "home", parameters: {} };
        const definition = present(revision.definition, "the definition");
        await store.mutate(store.serving(installation), {
            id: RequestId.create(Date.now()),
            calls: [
                installation.call("submit", {
                    ...installation.target(installed),
                    selection: { kind: "release", version: stacked.version },
                    build: { kind: "release", version: stacked.version, manifest: second },
                    evaluation: { ...evaluation, definition },
                }),
            ],
        });
    };
    const move = (moved: typeof carried) =>
        fixture.database.update(space.table).set({ stack: moved }).where(eq(space.table.id, home));

    // follow the first build, apply the second, then move the space's stack to a setting value
    const steps = [await followed()];
    await apply();
    steps.push(await followed());
    await move({ ...carried, settings: { [color]: "teal" } });
    const listed = await controller.list();
    steps.push(await followed());
    steps.push(await followed());
    const revisions = await fixture.database
        .select({ id: installationRevision.table.id })
        .from(installationRevision.table);

    const placed = {
        [color]: {
            setting: { packageId: ids.stackPackage, name: "color" },
            mode: "set",
            value: "teal",
        },
    };
    expect({ steps, listed, revisions: revisions.length }).toEqual({
        steps: [
            // follow the space's first build
            { manifest: first, settings: {}, listed: [] },
            // keep the person's apply of the second build
            { manifest: second, settings: {}, listed: [] },
            // submit the space's stack again once it places a setting value
            { manifest: first, settings: placed, listed: [] },
            // keep the revision of an unchanged stack
            { manifest: first, settings: placed, listed: [] },
        ],
        // list the home while its stack changed since its last submission
        listed: [home],
        // record one revision per submission
        revisions: 3,
    });
});

/** Record the account's access as a replica, as the machine's chain replica keeps it. */
async function replicateChain(fixture: StackFixture): Promise<void> {
    await fixture.database.insert(replica).values({
        name: `chain:${ids.account}`,
        scope: ids.account,
        tables: [accessRelationship[TABLE].sqlName],
    });
}
