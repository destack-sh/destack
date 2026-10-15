import { type DatabaseConnection, eq } from "@destack/db";
import { Catalog } from "@destack/locale";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, Identifier } from "@destack/schema";
import { declaration, installation, installationRevision } from "../object/index.ts";

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
