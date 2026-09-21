import {
    check,
    dialectSQL,
    foreignKey,
    identifier,
    integer,
    primaryKey,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { packageTable } from "./package.ts";
import { repository } from "./repository.ts";

/** Release records. */
export const release = table(
    "release",
    {
        /** The released package. */
        packageId: identifier("package_id", "package")
            .notNull()
            .references(() => packageTable.id, {
                onDelete: "restrict",
            }),
        /** The immutable calendar version. */
        version: text("version").notNull(),
        /** The SHA-256 digest of the published manifest. */
        manifest: text("manifest").notNull(),
        /** The immutable source repository reference. */
        repositoryId: identifier("repository_id", "repository")
            .notNull()
            .references(() => repository.id, { onDelete: "restrict" }),
        /** The source package directory at this commit. */
        directory: text("directory").notNull(),
        /** The full Git SHA-1 or SHA-256 commit identifier. */
        commit: text("commit").notNull(),
        /** Publication time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
    },
    (release) => [
        foreignKey({
            columns: [release.repositoryId, release.packageId],
            foreignColumns: [packageTable.repositoryId, packageTable.id],
        }).onDelete("restrict"),
        check(
            "release_commit",
            dialectSQL({
                sqlite: sql`length(${release.commit}) IN (40, 64) AND ${release.commit} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${release.commit}) IN (40, 64) AND (${release.commit} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
        check(
            "release_directory",
            sql`length(${release.directory}) > 0 AND substr(${release.directory}, 1, 1) <> '/' AND ${release.directory} <> '..' AND ${release.directory} NOT LIKE '../%' AND ${release.directory} NOT LIKE '%/../%' AND ${release.directory} NOT LIKE '%/..'`,
        ),
        primaryKey({ columns: [release.packageId, release.version] }),
        unique("release_manifest_version").on(release.packageId, release.version, release.manifest),
        check(
            "release_manifest",
            dialectSQL({
                sqlite: sql`length(${release.manifest}) = 64 AND ${release.manifest} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${release.manifest}) = 64 AND (${release.manifest} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
    ],
);

/** A persisted release record. */
export type Release = Select<typeof release>;
