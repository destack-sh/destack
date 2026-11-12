import type { DatabaseConnection } from "@destack/db";
import { type BuildReader, Lock, PackageManifest } from "@destack/package/manifest";
import type { Environment } from "@destack/environment";
import { Digest, Identifier } from "@destack/schema";
import { LATEST_TAG, packageObject, release, tag } from "@destack/registry/object";
import { ReleaseMethod } from "../server/release.ts";

/** The commit a test release records, the all-zero SHA-1. */
export const TEST_COMMIT = "0".repeat(40);

/** Forge's release type resolving releases by version or tag, as other packages read them. */
export const resolvedRelease = release.implement({
    methods: { resolve: (call) => ReleaseMethod.resolve(call) },
});

/** Record a build locked alone as a package's published release in a space, tagged. */
export async function publishBuild(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    accountId: Identifier<"account">,
    build: BuildReader,
    environment: Environment,
): Promise<void> {
    // store the package under its name, public to every reader, at the environment's time
    const now = environment.clock.now();
    const owner = PackageManifest.package(build.manifest);
    const manifest = await Digest.json(build.manifest, environment.crypto);
    const lock = await Lock.digest(Lock.of(manifest), environment.crypto);
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(packageObject.table)
                .values({
                    id: owner.id,
                    scope,
                    accountId,
                    name: owner.name.slice(owner.name.indexOf("/") + 1),
                    visibility: "public",
                    createdAt: now,
                    updatedAt: now,
                })
                .onConflictDoNothing(),
    );

    // store the release
    await database.transaction(
        async (transaction) =>
            await transaction.insert(release.table).values({
                id: Identifier.create("release", environment),
                scope,
                parentId: owner.id,
                version: owner.version,
                lock,
                commit: TEST_COMMIT,
                createdAt: now,
                updatedAt: now,
            }),
    );

    // point the latest tag at it
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(tag.table)
                .values({
                    id: Identifier.create("tag", environment),
                    scope,
                    parentId: owner.id,
                    name: LATEST_TAG,
                    version: owner.version,
                    createdAt: now,
                    updatedAt: now,
                })
                .onConflictDoUpdate({
                    target: [tag.table.parentId, tag.table.name],
                    set: { version: owner.version, updatedAt: now },
                }),
    );
}
