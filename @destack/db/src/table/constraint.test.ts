import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { integer, text } from "./column.ts";
import { check } from "./constraint.ts";
import { defineTable } from "./table.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";

/** Commits of a repository, each check one condition operator. */
const commit = defineTable(
    "constraint_commit",
    {
        /** The commit's key. */
        id: text("id").primaryKey(),
        /** The commit hash, 40 or 64 lowercase hex digits. */
        hash: text("hash").notNull(),
        /** The commit's label in lowercase letters, refusing the reserved `xn` and empty text. */
        label: text("label")
            .validate(schema.string().regex(/^[a-z]+$/u))
            .notNull(),
        /** The path the commit touches, absolute. */
        path: text("path").notNull(),
        /** The state of the commit's build. */
        state: text("state").notNull(),
        /** The build's manifest, present once it succeeded. */
        manifest: text("manifest"),
        /** When the commit was made. */
        createdAt: integer("created_at").notNull(),
        /** When the commit expires, after it was made. */
        expiresAt: integer("expires_at"),
    },
    {
        constraints: () => [
            check("constraint_commit_hash", {
                hash: { length: { in: [40, 64] }, characters: "0-9a-f" },
            }),
            check("constraint_commit_label", { label: { ne: "xn", NOT: { eq: "" } } }),
            check("constraint_commit_path", { path: { startsWith: "/", NOT: { endsWith: "_" } } }),
            check("constraint_commit_manifest", {
                OR: [
                    { state: "succeeded", manifest: { isNotNull: true } },
                    { state: { ne: "succeeded" }, manifest: { isNull: true } },
                ],
            }),
            check("constraint_commit_expiry", {
                OR: [
                    { expiresAt: { isNull: true } },
                    { expiresAt: { gt: { field: "createdAt" } } },
                ],
            }),
        ],
    },
);

/** A commit meeting every check. */
const KEPT = {
    id: "a",
    hash: "a".repeat(40),
    label: "main",
    path: "/src",
    state: "succeeded",
    manifest: "{}",
    createdAt: 1,
    expiresAt: 2,
};

test.for(TEST_DIALECTS)(
    "refuse rows breaking each check of conditions, comparing with values the column refuses, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [commit], { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;

        // insert a kept row, then one breaking each check
        await database.insert(commit).values(KEPT);
        const broken = [
            { hash: "A".repeat(40) },
            { hash: "a".repeat(41) },
            { label: "xn" },
            { path: "src" },
            { path: "/src_" },
            { manifest: null },
            { state: "failed" },
            { expiresAt: 1 },
        ];
        const refused = await Promise.all(
            broken.map(async (change, index) =>
                database
                    .insert(commit)
                    .values({ ...KEPT, ...change, id: `b${index}` })
                    .then(
                        () => "kept",
                        (error: unknown) => (error instanceof Error ? error.message : "thrown"),
                    ),
            ),
        );

        expect(refused).toEqual(broken.map(() => "a record fails a declared check"));
    },
);
