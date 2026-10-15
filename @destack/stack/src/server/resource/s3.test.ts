import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { TestDatabase } from "@destack/db/test";
import { aligned, found, schema, zip } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBucketHost } from "@destack/bucket/bun";
import { bucket } from "../../object/index.ts";
import { S3Credentials, S3Location, S3Signature } from "@destack/bucket/s3";
import { bucketDatabase } from "../../test/index.ts";
import { serveS3 } from "./s3.ts";
import { Derivation } from "@destack/identity";

/** The two spaces of the scenario, each keeping one bucket. */
const SPACES = [
    schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001"),
    schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
] as const;

/** The bucket of each space. */
const BUCKETS = [
    schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-00000000000a"),
    schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-00000000000b"),
] as const;

test("serve each space's buckets over S3 to its own credentials, concealing them from another space and refusing an unknown key", async () => {
    // keep a root secret per space and derive each space's credentials from it
    const roots = new Map<string, CryptoKey>(
        await Promise.all(
            SPACES.map(
                async (space) =>
                    [
                        space,
                        await crypto.subtle.importKey(
                            "raw",
                            crypto.getRandomValues(new Uint8Array(32)),
                            "HKDF",
                            false,
                            ["deriveBits"],
                        ),
                    ] as const,
            ),
        ),
    );
    const [first, second] = SPACES;
    const derived = await Promise.all(
        [first, first, second].map((space) =>
            S3Credentials.derive(Derivation.of(found(roots, space)), space),
        ),
    );

    // keep a bucket per space on a machine serving the derived credentials
    const directory = await mkdtemp(join(tmpdir(), "destack-s3-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        credentials: async (space) => {
            const root = roots.get(space);

            return root === undefined
                ? undefined
                : S3Credentials.derive(Derivation.of(root), space);
        },
    });
    const storage = await TestDatabase.create("sqlite", bucketDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const now = Date.now();
    for (const [space, bucketId] of zip(SPACES, BUCKETS)) {
        await storage.database.insert(bucket.table).values({
            id: bucketId,
            scope: space,
            name: "files",
            definitionPackageId: bucket.package.id,
            definitionVersion: bucket.package.version,
            definitionName: "files",
            spec: {},
            createdAt: now,
            updatedAt: now,
        });
        await (await buckets.open({ bucketId }, space)).put("notes/a.txt", space);
    }
    const server = serveS3(buckets, storage.database);

    // read the first space's file with its own credentials, its neighbour's, and an unknown key
    const reference = { scope: first, bucketId: BUCKETS[0] };
    const { location, credentials } = await buckets.locate(reference, "read");
    const read = async (signing: S3Credentials) => {
        const presigned = await new S3Signature({ region: location.region }).presign(
            new Request(S3Location.url(location, "notes/a.txt").href),
            signing,
            60,
            Date.now(),
        );
        const response = await server.fetch(presigned);
        const code = /<Code>([^<]+)<\/Code>/u.exec(await response.text())?.[1];

        return [response.status, code ?? "none"];
    };
    const outcomes = await Promise.all(
        [
            credentials,
            aligned(derived, 2),
            { accessKeyId: "AKIDUNKNOWN", secretAccessKey: "secret" },
        ].map(read),
    );

    // serve the space its own bucket, conceal it from a neighbour and refuse an unknown key
    expect(outcomes).toEqual([
        [200, "none"],
        [404, "NoSuchBucket"],
        [403, "InvalidAccessKeyId"],
    ]);
});
