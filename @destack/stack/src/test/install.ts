import type { Environment } from "@destack/environment";
import { type InstallationSelection } from "../declare/index.ts";
import { type DatabaseConnection, eq, Snapshot, Table } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, Identifier, present, schema } from "@destack/schema";
import { type Call, ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { Package, type PackageId } from "@destack/package";
import {
    build,
    deployment,
    installationRevision,
    instance,
    Rollout,
    rollout,
} from "../object/index.ts";
import stackPackage from "../package.ts";
import { Installation, installation, PackageDescription } from "@destack/object";
import { accessRelationship, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { residency, space } from "@destack/account/object";
import { PackageStore } from "@destack/build/store";
import { ids, spaceDatabase } from "./fixture.ts";
import { onTestFinished } from "@destack/test";
import type { ObjectStore } from "@destack/object/server";
import { workload } from "./build.ts";

/** Read the manifest digest a test installation records for a build, as a store records it. */
function manifestOf(reader: BuildReader, crypto: Environment["crypto"]): Promise<Digest> {
    return PackageStore.digest(reader.manifest, crypto);
}

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

/** Record an installed package and its rollout following a selection, as installing one does. */
export async function insertRollout(
    database: DatabaseConnection,
    installed: {
        readonly id: Identifier<"installation">;
        readonly scope: Identifier<"space">;
        readonly packageId: PackageId;
        readonly name: string;
        readonly selection: InstallationSelection;
        readonly installer: Subject;
        readonly origin?: "declared" | "installed";
        readonly capabilities?: readonly string[];
    },
    now: number,
): Promise<Rollout> {
    // write the installation, then the rollout following its selection under the same identifier, both by the installer
    const {
        id,
        scope,
        packageId,
        name,
        selection,
        installer,
        origin = "installed",
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
        await transaction
            .insert(installation.table)
            .values({ ...record, origin, packageId, name, capabilities: [...capabilities] });
        const [inserted] = await transaction
            .insert(rollout.table)
            .values({ ...record, selection })
            .returning();

        return present(inserted, `the rollout ${id}`);
    });
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

/** Install a build in a space as a stack does: its installation, release, revision and access. */
export async function installBuild(
    store: ObjectStore,
    scope: Identifier<"space">,
    reader: BuildReader,
    installer: Subject,
    options: {
        /** The optional capabilities the space's stack allows the installation. */
        readonly capabilities?: readonly string[];
    } = {},
): Promise<Identifier<"installation">> {
    // insert the installation under its package's name, at the world's time
    const { database, environment } = store;
    const now = environment.clock.now();
    const owner = PackageManifest.package(reader.manifest);
    const installationId = Identifier.create("installation", environment);
    const revisionId = Identifier.create("installation-revision", environment);
    const manifest = await manifestOf(reader, environment.crypto);
    const name = owner.name.slice(owner.name.indexOf("/") + 1);
    const selection = { kind: "release" as const, version: owner.version };

    // declare it in a residency's home, whose stack declares its installations, else install it
    const [homed] = database.replicates(residency.table)
        ? await Snapshot.live(database)
              .read(residency.table)
              .findMany({ where: { space: scope }, limit: 1 })
        : [];
    const origin = homed === undefined ? "installed" : "declared";
    await insertRollout(
        database,
        {
            id: installationId,
            scope,
            packageId: owner.id,
            name,
            selection,
            installer,
            origin,
            capabilities: options.capabilities ?? [],
        },
        now,
    );

    // record the release as the space's build, and the revision with the build's translations
    const release = { package: owner.id, version: owner.version, manifest };
    const pulled = await insertRelease(
        database,
        scope,
        release,
        Object.keys(reader.manifest.outputs),
        environment,
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                id: revisionId,
                scope,
                installationId,
                buildId: pulled.id,
                dependencies: [],
                digest: manifest,
                createdAt: now,
                updatedAt: now,
            }),
    );
    // follow the revision
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ revisionId, appliedRevisionId: revisionId })
                .where(eq(rollout.table.id, installationId)),
    );

    // grant what the package requests as the space's stack does, on the installer's authority
    const snapshot = Snapshot.live(database);
    const [row] = await snapshot.read(rollout.table).findMany({ where: { id: installationId } });
    const [target] = await Rollout.read(snapshot, [present(row, `rollout ${installationId}`)]);
    const served = principal.space.reference(Scope.universe.id, scope);
    const stack = await Installation.base(served, stackPackage, environment.crypto);
    const subject = principal.installation.reference(scope, stack.installation);
    const installed = present(target, `rollout ${installationId}`);
    const description = await PackageDescription.of(reader);
    const changes = await Rollout.access(snapshot, installed, description, { subject, now });
    for (const change of changes) {
        await store.mutate(change.caller, {
            id: RequestId.create(environment),
            calls: change.calls,
        });
    }

    // relate a residency home's declared installation, answering the residency as the universe does
    const set = Rollout.permissions(installed.installation, description, "installation");
    for (const call of await Rollout.relate(snapshot, installed, set)) {
        await answerResidency(database, scope, call, environment);
    }

    return installationId;
}

/** Answer a residency's touch or revoke as its universe does, on the home's replica. */
async function answerResidency(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    call: Call,
    environment: Pick<Environment, "clock" | "crypto" | "random">,
): Promise<void> {
    // find the installation's standing relation to the residency
    const { id, installation: installed } = ResidencyCall.parse(call.input);
    const object = residency.reference(Scope.universe.id, id);
    const subject = principal.installation.reference(scope, installed);
    const standing = await Snapshot.live(database)
        .read(accessRelationship)
        .findFirst({
            where: {
                objectKey: ObjectReference.key(object),
                relation: "installation",
                ...ObjectReference.columns("subject", subject),
            },
        });

    // relate a touched installation once
    if (call.method === "touch" && standing === undefined) {
        await new AccessFixture(database, environment).replicateRelationship(
            object,
            "installation",
            subject,
        );
    }
    // unrelate a revoked one
    else if (call.method === "revoke" && standing !== undefined) {
        await database.transaction(
            async (transaction) =>
                await transaction
                    .delete(accessRelationship)
                    .where(eq(accessRelationship.id, standing.id)),
        );
    }
}

/** The residency and installation of a residency's touch or revoke. */
const ResidencyCall = schema.looseObject({
    /** The residency. */
    id: schema.string().min(1),
    /** The installation the home declares. */
    installation: schema.identifier("installation"),
});

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
