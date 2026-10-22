import { type DatabaseConnection, eq } from "@destack/db";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, Identifier, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import type { PackageId } from "@destack/package";
import {
    build,
    module,
    declaration,
    symbol,
    catalog,
    deployment,
    installation,
    installationRevision,
    instance,
} from "../object/index.ts";
import { ids } from "./fixture.ts";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { onTestFinished } from "@destack/test";
import { BunGraphStorage } from "../bun/graph.ts";
import type { PackageImplementation } from "@destack/object/server";
import { serversOf } from "@destack/object/test";
import { workload } from "./build.ts";

/** Serve the index of the builds a test installs, as a space's stack does, until the test ends. */
export async function buildIndexServers(
    database: DatabaseConnection,
    builds: readonly BuildReader[],
): Promise<PackageImplementation[]> {
    return serversOf([build, module, declaration, symbol, catalog], {
        external: await buildIndex(database, builds),
    });
}

/** Index the builds a test installs, in a temporary directory until the test ends. */
async function buildIndex(
    database: DatabaseConnection,
    builds: readonly BuildReader[],
): Promise<BunGraphStorage> {
    // open each build by the manifest its installation records
    const byManifest = new Map(
        await Promise.all(builds.map(async (each) => [await manifestOf(each), each] as const)),
    );
    const directory = await mkdtemp(join(tmpdir(), "destack-stack-index-"));
    const index = new BunGraphStorage(
        database,
        {
            contains: async (manifest) => byManifest.has(manifest),
            contents: async (manifest) =>
                present(byManifest.get(manifest), `the build ${manifest}`),
            referrer: async () => undefined,
            file: () => Promise.reject(new TypeError("the installed builds carry no code index")),
        },
        directory,
    );

    // close the index and remove its files once the test ends
    onTestFinished(async () => {
        await index[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });

    return index;
}

/** Read the manifest digest a test installation records for a build. */
function manifestOf(reader: BuildReader): Promise<string> {
    return Digest.json(reader.manifest);
}

/** Keep a release as a space's build, pulled and succeeded, as creating a release build does. */
export async function insertRelease(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    release: { readonly package: PackageId; readonly version: string; readonly manifest: string },
    outputs: readonly string[] = [],
    now = Date.now(),
): Promise<{ readonly id: Identifier<"build">; readonly manifest: string }> {
    // record the build succeeded at the release's manifest, and its own scope
    const id = Identifier.create("build");
    const { manifest } = release;
    await database.insert(build.table).values({
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
    });
    await insertBuildScope(database, scope, id);

    return { id, manifest };
}

/** Keep a build's own scope inside its space's, as creating a build does. */
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
    await database.insert(Scope.table).values({
        scope: buildId,
        parent: spaceId,
        ancestors: [spaceId, ...parent.ancestors],
        partition: parent.partition,
        packageId: build.package.id,
        type: build.name,
    });
}

/** Install a build in a space as a stack does: its installation, its release build and revision. */
export async function installBuild(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    reader: BuildReader,
    now = Date.now(),
): Promise<Identifier<"installation">> {
    // keep the installation under its package's name
    const owner = PackageManifest.package(reader.manifest);
    const installationId = Identifier.create("installation");
    const revisionId = Identifier.create("installation-revision");
    const manifest = await manifestOf(reader);
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

    // keep the release as the space's build, and the revision with the build's translations
    const release = { package: owner.id, version: owner.version, manifest };
    const pulled = await insertRelease(
        database,
        scope,
        release,
        Object.keys(reader.manifest.outputs),
        now,
    );
    await database.insert(installationRevision.table).values({
        id: revisionId,
        scope,
        installationId,
        buildId: pulled.id,
        dependencies: [],
        digest: manifest,
        createdAt: now,
        updatedAt: now,
    });
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
    release: { readonly id: Identifier<"build"> },
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
    release: { readonly id: Identifier<"build"> },
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
        buildId: release.id,
        dependencies: [],
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
