import { MAIN_DATABASE } from "@destack/stack/declare";
import { eq, isNotNull } from "@destack/db";
import { installation } from "@destack/object";
import { Package } from "@destack/package";
import { PackageStore } from "@destack/build/store";
import { MemoryBuild } from "@destack/package/test";
import { Identifier, nth, present, schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/object/test";
import { build, database, rollout, installationRevision } from "../src/object/index.ts";
import stackPackage from "../src/package.ts";
import { ids, spaceDatabase, TestSpace } from "../src/test/index.ts";

/** The stack package the spaces install. */
const stacked = Package.parse({ id: ids.stackPackage, name: "@test/stack", version: "2026.9.0" });

/** The color a space's stack sets, which its package declares. */
const color = defineSetting({
    name: "color",
    title: "Color",
    description: "The space's color.",
    schema: schema.string(),
    default: "blue",
    scope: "space",
    overrides: [],
    apply: "immediate",
});

test("give birth to placed spaces from their replicated rows with main in their own storage, refuse a stack declaring main, and resume a stack left unsubmitted", async () => {
    // publish a stack without main and one declaring it, built in a world of its own
    const { crypto } = testSimulation().environment("test");
    const main = spaceDatabase.declare({ placement: { provider: "postgresql" } });
    const stackBuild = await MemoryBuild.of(
        stacked,
        [
            {
                kind: "stack",
                kindPackageId: stackPackage.id,
                name: "home",
                description: { installations: {} },
            },
            {
                kind: "stack",
                kindPackageId: stackPackage.id,
                name: "placed",
                description: { databases: { main }, installations: {} },
            },
        ],
        crypto,
    );
    const manifest = await PackageStore.digest(stackBuild.manifest, crypto);
    const carrying = (name: string) => ({ package: stacked, manifest, export: name, settings: {} });

    // create a space carrying each stack at the universe, born on the residency's machine
    const home = await TestSpace.open({ stack: carrying("home"), releases: [stackBuild] });
    const refused = await TestSpace.refusals(
        TestSpace.open({
            world: home,
            owner: home.owner,
            account: ids.account,
            id: Identifier.create("space", home.environment),
            name: "placed",
            stack: carrying("placed"),
            releases: [stackBuild],
        }),
    );

    // forget the home's submission as a crash before it would, and give birth to it again
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ revisionId: null, spaceStack: null })
                .where(eq(rollout.table.scope, home.id)),
    );
    const born = await home.serve();
    await born[Symbol.asyncDispose]();

    // read the home's main placement and its stack installation
    const mains = await home.database
        .select({ placement: database.table.placement })
        .from(database.table)
        .where(eq(database.table.name, MAIN_DATABASE));
    const stacks = await home.database
        .select({ scope: rollout.table.scope, revisionId: rollout.table.revisionId })
        .from(rollout.table)
        .innerJoin(installation.table, eq(installation.table.id, rollout.table.id))
        .where(isNotNull(installation.table.export));
    expect({
        mains: mains.map((entry) => entry.placement),
        stacks: stacks.map((entry) => [entry.scope, entry.revisionId !== null]),
        refused,
    }).toEqual({
        // store main in the space's own storage
        mains: [{ provider: "space" }],
        // install the home's stack once, submitting it again after the crash
        stacks: [[home.id, true]],
        // refuse the stack declaring main
        refused: [
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                "stack placed declares main, but every space already has it",
            ],
        ],
    });
});

test("retain a person's own apply of a placed space's stack, and submit the space's stack again once it changes, once only", async () => {
    // publish two builds of the stack declaring its color, the second exporting one more, and create a space carrying the first
    const { crypto } = testSimulation().environment("test");
    const declaring = (names: readonly string[]) =>
        MemoryBuild.of(
            stacked,
            [
                {
                    kind: "setting",
                    kindPackageId: settingPackage.id,
                    name: color.name,
                    // declare it as the stack package's own
                    description: { ...describeSetting(color), package: stacked },
                },
                ...names.map((name) => ({
                    kind: "stack",
                    kindPackageId: stackPackage.id,
                    name,
                    description: { installations: {} },
                })),
            ],
            crypto,
        );
    const builds = [await declaring(["home"]), await declaring(["home", "studio"])];
    const first = await PackageStore.digest(nth(builds, 0).manifest, crypto);
    const second = await PackageStore.digest(nth(builds, 1).manifest, crypto);
    const carried = { package: stacked, manifest: first, export: "home", settings: {} };
    const fixture = await TestSpace.open({ stack: carried, releases: builds });

    // serve the space, its machine's placement submitting its stack
    const served = await fixture.serve();

    // read the space's stack installation and the revision it follows
    const stack = async () => {
        const [row] = await fixture.database
            .select({ installed: rollout.table, revision: installationRevision.table })
            .from(rollout.table)
            .innerJoin(
                installationRevision.table,
                eq(installationRevision.table.id, rollout.table.revisionId),
            )
            .where(eq(rollout.table.scope, fixture.id));

        return present(row, "the space's stack");
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

    // read the build and settings the space's stack follows once the space settles
    const colorKey = `${ids.stackPackage}/${color.name}`;
    const followed = async () => {
        await fixture.settle(served);
        const { revision } = await stack();

        return {
            manifest: await pulledManifest(revision.buildId),
            settings: present(revision.definition, "the definition")["settings"],
        };
    };

    // apply the second build as the space's owner does, without the space's stack
    const apply = async () => {
        const { installed, revision } = await stack();
        const owner = served.as(fixture.owner.subject).client(stackPackage);
        await owner.rollout.submit({
            spaceId: fixture.id,
            id: installed.id,
            requestId: RequestId.create(fixture.machine.environment),
            selection: { kind: "release", version: stacked.version },
            build: { kind: "release", version: stacked.version, manifest: second },
            evaluation: {
                export: "home",
                parameters: {},
                definition: present(revision.definition, "the definition"),
            },
        });
    };

    // move the space's stack to a setting value as the platform does
    const move = () =>
        fixture.universe.upgrade(
            { accountId: ids.account, id: fixture.id },
            { ...carried, settings: { [colorKey]: "teal" } },
        );

    // follow the first build, apply the second, then move the space's stack to a setting value
    const steps = [await followed()];
    await apply();
    steps.push(await followed());
    await move();
    steps.push(await followed());
    steps.push(await followed());
    const revisions = await fixture.database
        .select({ id: installationRevision.table.id })
        .from(installationRevision.table)
        .where(eq(installationRevision.table.scope, fixture.id));

    const placed = {
        [colorKey]: { packageId: ids.stackPackage, name: color.name, mode: "set", value: "teal" },
    };
    expect({ steps, revisions: revisions.length }).toEqual({
        steps: [
            // follow the space's first build
            { manifest: first, settings: {} },
            // retain the owner's apply of the second build
            { manifest: second, settings: {} },
            // submit the space's stack again once it places a setting value
            { manifest: first, settings: placed },
            // retain the revision of an unchanged stack
            { manifest: first, settings: placed },
        ],
        // record one revision per submission
        revisions: 3,
    });
});
