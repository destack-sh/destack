import {
    installation,
    build,
    Build,
    deployment,
    installationRevision,
    instance,
    Installation,
    type InstallationRevision,
} from "@destack/stack/object";
import type { Environment } from "@destack/environment";
import { type Database, eq, isNotNull, Table } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { Digest, Identifier, present, schema } from "@destack/schema";
import { ObjectReference, Scope, type Subject } from "@destack/sync";
import { release } from "@destack/registry/object";
import type { PackageId } from "@destack/package";
import stackPackage from "@destack/stack/package";
import { SYNCED } from "@destack/object";
import { space } from "@destack/account/object";
import { ids, spaceDatabase } from "./space.ts";
import { onTestFinished } from "@destack/test";
import { workload } from "./build.ts";

/** Open a migrated space database with the space's scope and row, in a world of its own. */
export async function openSpaceDatabase(declared: Database = spaceDatabase): Promise<{
    readonly database: DatabaseConnection;
    readonly crypto: Environment["crypto"];
    readonly now: number;
}> {
    // open the space's database
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(declared, { environment: world, isMigrated: true });
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
                    packageId: stackPackage.id,
                    version: { kind: "release", version: stackPackage.version },
                    export: "default",
                    settings: {},
                    routes: {},
                },
                createdAt: now,
                updatedAt: now,
            }),
        );
    });

    return { database, crypto: world.crypto, now };
}

/** Record an installed package following a row, as installing one does. */
export async function insertInstallation(
    database: DatabaseConnection,
    installed: {
        readonly id: Identifier<"installation">;
        readonly scope: Identifier<"space">;
        readonly packageId: PackageId;
        readonly name: string;
        readonly version: ObjectReference | undefined;
        readonly directory?: string;
        readonly installer: Subject;
        readonly capabilities?: readonly string[];
    },
    now: number,
): Promise<Installation> {
    // write the installation following its selection, by the installer
    const {
        id,
        scope,
        packageId,
        name,
        version,
        directory,
        installer,
        capabilities = [],
    } = installed;
    const record = {
        id,
        scope,
        createdAt: now,
        updatedAt: now,
        ...ObjectReference.columns("createdBy", installer),
        ...ObjectReference.columns("updatedBy", installer),
    };

    return await database.transaction(async (transaction) => {
        const [inserted] = await transaction
            .insert(installation.table)
            .values({
                ...record,
                packageId,
                name,
                ...ObjectReference.columns("version", version),
                directory: directory ?? null,
                capabilities: [...capabilities],
            })
            .returning();

        return present(inserted, `the installation ${id}`);
    });
}

/** Read an installation. */
export async function readInstalled(
    database: DatabaseConnection,
    id: Identifier<"installation">,
): Promise<Installation> {
    const [row] = await database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, id));

    return present(row, `the installation ${id}`);
}

/** The Synced condition a stack refusal records on an installation at its generation. */
export function refusedCondition(installed: Installation, reason: string, message: string) {
    const { generation, conditions } = installed;

    return {
        status: "false",
        reason: "ReconcileError",
        message: `@destack/stack ${reason}: ${message}`,
        observedGeneration: generation,
        lastTransitionAt: conditions[SYNCED]?.lastTransitionAt,
    };
}

/** Read the space's one stack installation, the one its birth gives it. */
export async function readStack(database: DatabaseConnection): Promise<Installation> {
    // read the installation exporting the space's stack
    const [stack] = await database
        .select()
        .from(installation.table)
        .where(isNotNull(installation.table.export));

    return present(stack, "the space's stack");
}

/** Record a release as a space's build, pulled and succeeded, as creating a release build does. */
export async function insertRelease(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    released: { readonly package: PackageId; readonly version: string; readonly image: Digest },
    outputs: readonly string[],
    environment: Environment,
): Promise<{ readonly id: Identifier<"build">; readonly image: Digest }> {
    // answer the space's live build of the release, as creating one does
    const { image } = released;
    const source = { kind: "release" as const, ...released };
    const live = await Build.find(database.snapshot(), scope, {
        source,
        outputs: null,
        version: null,
        destack: import.meta.destack.package.version,
    });
    if (live !== undefined) {
        return { id: live.id, image };
    }

    // else record the build succeeded at the release's image
    const now = environment.clock.now();
    const id = Identifier.create("build", environment);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values({
                id,
                scope,
                source,
                key: Build.key({
                    source,
                    outputs: null,
                    version: null,
                    destack: import.meta.destack.package.version,
                }),
                outputs: [...outputs],
                state: "succeeded",
                image,
                startedAt: now,
                finishedAt: now,
                createdAt: now,
                updatedAt: now,
            }),
    );
    await insertBuildScope(database, scope, id);

    return { id, image };
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

/** A release row of the notes package, which tests of a database alone follow and never resolve. */
export const NOTES_RELEASE = release.reference(
    Scope.universe.id,
    "release-01996ab0-0000-7000-8000-0000000000e3",
);

/** The notes installation's one running instance. */
export const NOTES_INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-0000000000e2");

/** Install the notes release in the space and run one instance of it, answering the instance. */
export async function runNotes(
    database: DatabaseConnection,
    released: {
        readonly id: Identifier<"build">;
        readonly recorded: Pick<InstallationRevision, "release" | "declarations">;
    },
    version: ObjectReference,
    installer: Subject,
    environment: Environment,
): Promise<Identifier<"instance">> {
    // install the release, deploy it and run its instance
    const revisionId = await installNotes(database, released, version, installer, environment);
    await deployNotes(database, revisionId, environment);

    return NOTES_INSTANCE;
}

/** Install the notes package in the space, following its release's revision. */
async function installNotes(
    database: DatabaseConnection,
    released: {
        readonly id: Identifier<"build">;
        readonly recorded: Pick<InstallationRevision, "release" | "declarations">;
    },
    version: ObjectReference,
    installer: Subject,
    environment: Environment,
): Promise<Identifier<"installation-revision">> {
    // record the notes installation in the space, at the world's time
    const now = environment.clock.now();
    const notes = { id: ids.notes, scope: ids.space, packageId: ids.package, name: "notes" };
    await insertInstallation(database, { ...notes, version, installer }, now);

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
                buildId: released.id,
                dependencies: [],
                states: {},
                ...released.recorded,
                generation: 1,
                bindings: [],
                digest: Digest.parse("e".repeat(64)),
            }),
    );
    await database.transaction(async (transaction) => {
        await transaction
            .update(installation.table)
            .set({ targetRevisionId: revisionId, appliedRevisionId: revisionId })
            .where(eq(installation.table.id, ids.notes));
    });

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
