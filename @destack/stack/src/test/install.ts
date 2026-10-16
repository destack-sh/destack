import { type DatabaseConnection, eq } from "@destack/db";
import { Catalog } from "@destack/locale";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, Identifier, schema } from "@destack/schema";
import type { InstallationBuild } from "../declare/installation.ts";
import {
    declaration,
    deployment,
    installation,
    installationRevision,
    instance,
} from "../object/index.ts";
import { ids } from "./fixture.ts";
import { workload } from "./build.ts";

/** Install a build in a space as a stack does: its installation, revision and declarations. */
export async function installBuild(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    build: BuildReader,
    now = Date.now(),
): Promise<Identifier<"installation">> {
    // keep the installation under its package's name
    const { package: owner } = build.manifest;
    const installationId = Identifier.create("installation");
    const revisionId = Identifier.create("installation-revision");
    const manifest = await Digest.json(build.manifest);
    await database.insert(installation.table).values({
        id: installationId,
        scope,
        packageId: owner.id,
        role: "application",
        name: owner.name.slice(owner.name.indexOf("/") + 1),
        selection: { kind: "release", version: owner.version },
        createdAt: now,
        updatedAt: now,
    });

    // keep the revision with the build's translations, and each of its declarations
    await database.insert(installationRevision.table).values({
        id: revisionId,
        scope,
        installationId,
        build: { kind: "release", version: owner.version, manifest },
        views: {},
        settings: [],
        catalogs: await Catalog.read(build),
        digest: manifest,
        createdAt: now,
        updatedAt: now,
    });
    for (const declared of await build.declarations()) {
        await database.insert(declaration.table).values({
            id: Identifier.create("declaration"),
            scope,
            revisionId,
            installationId,
            packageId: owner.id,
            kindPackageId: declared.package,
            kind: declared.kind,
            name: declared.name,
            description: declared.description,
            createdAt: now,
            updatedAt: now,
        });
    }

    // follow the revision
    await database
        .update(installation.table)
        .set({ revisionId, appliedRevisionId: revisionId })
        .where(eq(installation.table.id, installationId));

    return installationId;
}

/** The notes installation's one running instance. */
export const NOTES_INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-0000000000e2");

/** Install the notes release in the space and run one instance of it, answering the instance. */
export async function runNotes(
    database: DatabaseConnection,
    release: InstallationBuild,
    now: number,
): Promise<Identifier<"instance">> {
    // install the release, deploy it and run its instance
    const revisionId = await installNotes(database, release, now);
    await deployNotes(database, revisionId, now);

    return NOTES_INSTANCE;
}

/** Install the notes package in the space, following its release's revision. */
async function installNotes(
    database: DatabaseConnection,
    release: InstallationBuild,
    now: number,
): Promise<Identifier<"installation-revision">> {
    // record the notes installation in the space
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });

    // follow the installation's release
    const revisionId = Identifier.create("installation-revision");
    await database.insert(installationRevision.table).values({
        id: revisionId,
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        installationId: ids.notes,
        build: release,
        views: {},
        settings: [],
        catalogs: [],
        digest: "e".repeat(64),
    });
    await database
        .update(installation.table)
        .set({ revisionId, appliedRevisionId: revisionId })
        .where(eq(installation.table.id, ids.notes));

    return revisionId;
}

/** Deploy the notes installation's revision and run one instance of it. */
async function deployNotes(
    database: DatabaseConnection,
    revisionId: Identifier<"installation-revision">,
    now: number,
): Promise<void> {
    // deploy the revision
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const deploymentId = Identifier.create("deployment");
    await database.insert(deployment.table).values({
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
    });

    // run one instance of it
    const machineId = schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-0000000000e1");
    await database.insert(instance.table).values({
        ...record,
        id: NOTES_INSTANCE,
        deploymentId,
        machineId,
        status: "running",
        observedAt: now,
        startedAt: now,
    });
}
