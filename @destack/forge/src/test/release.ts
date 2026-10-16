import type { DatabaseConnection } from "@destack/db";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, Identifier } from "@destack/schema";
import { LATEST_TAG, packageObject, release, tag } from "../object/index.ts";
import { ReleaseMethod } from "../server/release.ts";

/** The commit a test release records, the all-zero SHA-1. */
const TEST_COMMIT = "0".repeat(40);

/** Forge's release type resolving releases by version or tag, as other packages read them. */
export const resolvedRelease = release.implement({
    methods: { resolve: (call) => ReleaseMethod.resolve(call) },
});

/** Record a build as a package's published release in a space, tagged, as publishing it does. */
export async function publishBuild(
    database: DatabaseConnection,
    scope: Identifier<"space">,
    accountId: Identifier<"account">,
    build: BuildReader,
    now = Date.now(),
): Promise<void> {
    // store the package under its name, public to every reader
    const { package: owner } = build.manifest;
    const manifest = await Digest.json(build.manifest);
    await database
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
        .onConflictDoNothing();

    // store the release with the build's declarations and an empty archive
    await database.insert(release.table).values({
        id: Identifier.create("release"),
        scope,
        parentId: owner.id,
        version: owner.version,
        manifest,
        commit: TEST_COMMIT,
        distribution: {
            digest: manifest,
            shasum: TEST_COMMIT,
            integrity: "",
            size: 0,
            unpackedSize: 0,
            fileCount: 0,
        },
        metadata: {
            name: owner.name,
            version: owner.version,
            type: "module",
            exports: {},
            dependencies: {},
            optionalDependencies: {},
        },
        declarations: [...(await build.declarations())],
        createdAt: now,
        updatedAt: now,
    });

    // point the latest tag at it
    await database
        .insert(tag.table)
        .values({
            id: Identifier.create("tag"),
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
        });
}
