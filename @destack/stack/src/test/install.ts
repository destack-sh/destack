import type { Environment } from "@destack/environment";
import { type DatabaseConnection, eq, isNotNull, Snapshot, Table } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { Digest, Identifier, present, schema } from "@destack/schema";
import { ObjectReference, Scope, type Subject } from "@destack/sync";
import { Package, type PackageId } from "@destack/package";
import {
    build,
    deployment,
    installationRevision,
    instance,
    type InstallationRollout,
    Rollout,
    rollout,
} from "../object/index.ts";
import stackPackage from "../package.ts";
import { installation, type InstallationSelection } from "@destack/object";
import { space } from "@destack/account/object";
import { ids, spaceDatabase } from "./space.ts";
import { onTestFinished } from "@destack/test";
import { workload } from "./build.ts";

/** Open a migrated space database with the space's scope and row, in a world of its own. */
export async function openSpaceDatabase(): Promise<{
    readonly database: DatabaseConnection;
    readonly crypto: Environment["crypto"];
    readonly now: number;
}> {
    // open the space's database
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(spaceDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const now = world.clock.now();

    // record the space inside its account
    await database.transaction(async (transaction) => {
        await transaction.insert(Scope.table).values({
            scope: ids.space,
            parent: ids.account,
            ancestors: [ids.account, Scope.universe.id],
            partition: ids.space,
            packageId: space.package.id,
            type: space.name,
        });
        await transaction.insert(space.table).values(
            Table.complete(space.table, {
                id: ids.space,
                scope: ids.account,
                name: "notes",
                stack: {
                    package: Package.of(stackPackage),
                    manifest: await Digest.json({}, world.crypto),
                    export: "default",
                    settings: {},
                },
                createdAt: now,
                updatedAt: now,
            }),
        );
    });

    return { database, crypto: world.crypto, now };
}

/** Record an installed package following a selection and its rollout, as installing one does. */
export async function insertRollout(
    database: DatabaseConnection,
    installed: {
        readonly id: Identifier<"installation">;
        readonly scope: Identifier<"space">;
        readonly packageId: PackageId;
        readonly name: string;
        readonly selection: InstallationSelection;
        readonly installer: Subject;
        readonly capabilities?: readonly string[];
    },
    now: number,
): Promise<Rollout> {
    // write the installation, then the rollout following its selection under the same identifier, both by the installer
    const { id, scope, packageId, name, selection, installer, capabilities = [] } = installed;
    const record = {
        id,
        scope,
        createdAt: now,
        updatedAt: now,
        ...ObjectReference.columns("createdBy", installer),
        ...ObjectReference.columns("updatedBy", installer),
    };

    return await database.transaction(async (transaction) => {
        await transaction
            .insert(installation.table)
            .values({ ...record, packageId, name, selection, capabilities: [...capabilities] });
        const [inserted] = await transaction.insert(rollout.table).values(record).returning();

        return present(inserted, `the rollout ${id}`);
    });
}

/** Read an installation with its rollout. */
export async function readInstalled(
    database: DatabaseConnection,
    id: Identifier<"installation">,
): Promise<InstallationRollout> {
    const [row] = await Rollout.installed(
        Snapshot.live(database),
        await database.select().from(installation.table).where(eq(installation.table.id, id)),
    );

    return present(row, `the installation ${id}`);
}

/** Read the space's one stack installation with its rollout, the one its birth gives it. */
export async function readStack(database: DatabaseConnection): Promise<InstallationRollout> {
    // read the installation exporting the space's stack, with its rollout
    const [stacked] = await database
        .select({ id: installation.table.id })
        .from(installation.table)
        .where(isNotNull(installation.table.export));
    const rows = await database
        .select()
        .from(rollout.table)
        .where(eq(rollout.table.id, present(stacked, "the space's stack").id));
    const [stack] = await Rollout.read(Snapshot.live(database), rows);

    return present(stack, "the space's stack");
}

/** Record a release as a space's build, pulled and succeeded, as creating a release build does. */
export async function insertRelease(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    release: { readonly package: PackageId; readonly version: string; readonly manifest: Digest },
    outputs: readonly string[],
    environment: Environment,
): Promise<{ readonly id: Identifier<"build">; readonly manifest: Digest }> {
    // record the build succeeded at the release's manifest, and its own scope, at the world's time
    const now = environment.clock.now();
    const id = Identifier.create("build", environment);
    const { manifest } = release;
    await database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values({
                id,
                scope,
                source: { kind: "release", ...release },
                outputs: [...outputs],
                state: "succeeded",
                manifest,
                startedAt: now,
                finishedAt: now,
                createdAt: now,
                updatedAt: now,
            }),
    );
    await insertBuildScope(database, scope, id);

    return { id, manifest };
}

/** Insert a build's own scope inside its space's, as creating a build does. */
export async function insertBuildScope(
    database: DatabaseConnection,
    spaceId: Identifier<"space">,
    buildId: Identifier<"build">,
): Promise<void> {
    const [spaceScope] = await database
        .select()
        .from(Scope.table)
        .where(eq(Scope.table.scope, spaceId));
    const parent = present(spaceScope, `the scope of ${spaceId}`);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(Scope.table).values({
                scope: buildId,
                parent: spaceId,
                ancestors: [spaceId, ...parent.ancestors],
                partition: parent.partition,
                packageId: build.package.id,
                type: build.name,
            }),
    );
}

/** The notes installation's one running instance. */
export const NOTES_INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-0000000000e2");

/** Install the notes release in the space and run one instance of it, answering the instance. */
export async function runNotes(
    database: DatabaseConnection,
    release: { readonly id: Identifier<"build"> },
    installer: Subject,
    environment: Environment,
): Promise<Identifier<"instance">> {
    // install the release, deploy it and run its instance
    const revisionId = await installNotes(database, release, installer, environment);
    await deployNotes(database, revisionId, environment);

    return NOTES_INSTANCE;
}

/** Install the notes package in the space, following its release's revision. */
async function installNotes(
    database: DatabaseConnection,
    release: { readonly id: Identifier<"build"> },
    installer: Subject,
    environment: Environment,
): Promise<Identifier<"installation-revision">> {
    // record the notes installation in the space, at the world's time
    const now = environment.clock.now();
    const selection = { kind: "release" as const, version: "2026.9.0" };
    const notes = { id: ids.notes, scope: ids.space, packageId: ids.package, name: "notes" };
    await insertRollout(database, { ...notes, selection, installer }, now);

    // follow the installation's release
    const revisionId = Identifier.create("installation-revision", environment);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                id: revisionId,
                scope: ids.space,
                createdAt: now,
                updatedAt: now,
                installationId: ids.notes,
                buildId: release.id,
                dependencies: [],
                states: {},
                digest: Digest.parse("e".repeat(64)),
            }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ revisionId, appliedRevisionId: revisionId })
                .where(eq(rollout.table.id, ids.notes)),
    );

    return revisionId;
}

/** Deploy the notes installation's revision and run one instance of it. */
async function deployNotes(
    database: DatabaseConnection,
    revisionId: Identifier<"installation-revision">,
    environment: Environment,
): Promise<void> {
    // deploy the revision at the world's time
    const now = environment.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const deploymentId = Identifier.create("deployment", environment);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(deployment.table).values({
                ...record,
                id: deploymentId,
                installationId: ids.notes,
                packageId: ids.package,
                revisionId,
                output: "server",
                workload: "main",
                runtime: "bun",
                release: "2026.9.0",
                description: workload,
                activatedAt: now,
            }),
    );

    // run one instance of it
    const machineId = schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-0000000000e1");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(instance.table).values({
                ...record,
                id: NOTES_INSTANCE,
                deploymentId,
                machineId,
                status: "running",
                observedAt: now,
                startedAt: now,
            }),
    );
}
