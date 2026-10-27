import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { nth, found, schema, zip } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBucketHost } from "@destack/bucket/bun";
import { bucket } from "../../object/index.ts";
import { S3Credentials, S3Location, S3Signature } from "@destack/bucket/s3";
import { bucketDatabase } from "../../test/index.ts";
import { serveS3 } from "./s3.ts";
import { Derivation } from "@destack/identity";

/** The bytes of each space's root secret. */
const ROOT_BYTES = 32;

/** The two spaces of the scenario, each with one bucket. */
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
    // add a root secret per space in the test's world, and derive each space's credentials from it
    const world = testSimulation().environment("test");
    const { clock, crypto } = world;
    const roots = new Map<string, CryptoKey>(
        await Promise.all(
            SPACES.map(
                async (space) =>
                    [
                        space,
                        await crypto.importKey(
                            "raw",
                            world.random.bytes(ROOT_BYTES),
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
            S3Credentials.derive(Derivation.of(found(roots, space), crypto), space),
        ),
    );

    // add a bucket per space on a machine serving the derived credentials, in the world
    const directory = await world.files.makeTemporaryDirectory("destack-s3-");
    onTestFinished(() => world.files.remove(directory, { recursive: true }));
    await using buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost:4200"),
        region: "local",
        environment: world,
        credentials: async (space) => {
            const root = roots.get(space);

            return root === undefined
                ? undefined
                : S3Credentials.derive(Derivation.of(root, crypto), space);
        },
    });
    const storage = await TestDatabase.create(bucketDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const now = clock.now();
    for (const [space, bucketId] of zip(SPACES, BUCKETS)) {
        await storage.database.transaction(
            async (transaction) =>
                await transaction.insert(bucket.table).values({
                    id: bucketId,
                    scope: space,
                    name: "files",
                    definitionPackageId: bucket.package.id,
                    definitionVersion: bucket.package.version,
                    definitionName: "files",
                    spec: {},
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        await (await buckets.open({ bucketId }, space)).put("notes/a.txt", space);
    }
    const server = serveS3(buckets, storage.database, world);

    // read the first space's file with its own credentials, its neighbour's, and an unknown key
    const reference = { scope: first, bucketId: BUCKETS[0] };
    const { location, credentials } = await buckets.locate(reference, "read");
    const read = async (signing: S3Credentials) => {
        const presigned = await new S3Signature({ region: location.region, crypto }).presign(
            new Request(S3Location.url(location, "notes/a.txt").href),
            signing,
            60,
            clock.now(),
        );
        const response = await server.fetch(presigned);
        const code = /<Code>([^<]+)<\/Code>/u.exec(await response.text())?.[1];

        return [response.status, code ?? "none"];
    };
    const outcomes = await Promise.all(
        [
            credentials,
            nth(derived, 2),
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
