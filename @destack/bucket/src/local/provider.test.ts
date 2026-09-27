import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { resource, space } from "@destack/space/stack";
import { expect, test } from "@destack/test";
import { bucket, bucketDatabase } from "../stack/index.ts";
import { localBucketProvider } from "./index.ts";

/** Register a provisioned bucket for access decisions and remove it with its files. */
test("register provisioned buckets and remove destroyed ones", async () => {
    const regional = await TestDatabase.create(TEST_DIALECTS.at(-1)!, bucketDatabase, {
        isMigrated: true,
    });
    const { database } = regional;
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-provider-"));
    try {
        // record a space and its bucket resource
        const now = Date.now();
        const record = {
            id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000002"),
            scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
            kind: "bucket",
            spec: {},
            reference: null,
        };
        await database.insert(space).values({
            id: record.scope,
            scope: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000003"),
            name: "provider",
            authorityRegionId: identifier("region").parse(
                "region-01996ab0-0000-7000-8000-000000000004",
            ),
            authorityEpoch: 1,
            createdAt: now,
            updatedAt: now,
        });
        await database.insert(resource).values({
            id: record.id,
            scope: record.scope,
            name: "files",
            kind: "bucket",
            definitionPackageId: identifier("package").parse(
                "package-01996ab0-0000-7000-8000-000000000005",
            ),
            definitionVersion: "2026.9.0",
            definitionName: "files",
            spec: {},
            createdAt: now,
            updatedAt: now,
        });

        // provision twice, registering the bucket once
        const provider = localBucketProvider(database, directory);
        const provision = await provider.provision(record);
        expect(await provider.provision(record)).toEqual(provision);
        expect(await database.select().from(bucket)).toEqual([
            { resourceId: record.id, scope: record.scope, kind: "bucket" },
        ]);

        // destroy the files and the registration
        await provider.destroy({ ...record, reference: provision.reference });
        expect(await database.select().from(bucket)).toEqual([]);
        expect(await readdir(join(directory, record.scope))).toEqual([]);
    } finally {
        await rm(directory, { recursive: true });
        await regional.close();
    }
});
