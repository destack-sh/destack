import { createHash } from "node:crypto";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { defineDatabase, type DatabaseConnection, eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { ObjectServer } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/service/control";
import { testCallKey } from "@destack/service/test";
import { ConsumerController, serveObjects } from "@destack/space/server";
import { spaceCopies, spaceTables } from "@destack/space/stack";
import { ids, openDirectory, SpaceFixture } from "@destack/space/test";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalBucketHost } from "../local/index.ts";
import { bucket } from "../object/index.ts";
import { bucketProvider } from "../provider/index.ts";
import { bucketDatabase } from "../stack/index.ts";
import { implementBucket } from "./server.ts";

/** A cell's space database copying the buckets the bucket service keeps. */
const cellDatabase = defineDatabase({
    name: "space",
    tables: spaceTables,
    copies: [...spaceCopies, bucket.table],
});

/** Open a host's buckets in a temporary directory removed after the test. */
async function openBuckets(): Promise<LocalBucketHost> {
    const directory = await mkdtemp(join(tmpdir(), "destack-bucket-service-"));
    const buckets = new LocalBucketHost({
        directory,
        endpoint: new URL("http://s3.localhost"),
        region: "local",
        credentials: async () => ({ accessKeyId: "SERVICE", secretAccessKey: "service-secret" }),
    });
    onTestFinished(async () => {
        await buckets[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });

    return buckets;
}

/** Run some controllers over a database until the test ends, failing it on a reported failure. */
function run(database: DatabaseConnection, controllers: readonly Controller[]): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

test("provision the bucket a space requests through a sent call, and copy its record back to the space", async () => {
    // serve a space copying buckets, and the bucket service following the space's rows
    const fixture = await SpaceFixture.open({ database: cellDatabase });
    const reached = await openDirectory();
    const storage = await TestDatabase.create("sqlite", bucketDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const buckets = await openBuckets();
    let spaces: ObjectServer | undefined;
    const kept = implementBucket({
        database: storage.database,
        callKey: testCallKey,
        directory: reached.directory,
        providers: [bucketProvider(buckets)],
        machine: ids.machine,
        cell: ids.region,
        spaces: {
            stream: (subscription, signal) =>
                present(spaces, "the space service").uplink.stream(subscription, signal),
            receive: (mutation) => present(spaces, "the space service").uplink.receive(mutation),
        },
    });
    const space = serveObjects(await fixture.options({ ...reached, sources: [kept.objects] }));
    spaces = space;

    // request a bucket in the space through a call sent to the bucket service
    const bucketId = schema.identifier("bucket").parse(bucket.generateId());
    const now = Date.now();
    await fixture.database.transaction((transaction) =>
        space.change({ database: transaction, scope: ids.space, now }, bucket, "create", {
            id: bucketId,
            name: "files",
            definitionPackageId: bucket.package.id,
            definitionVersion: bucket.package.version,
            definitionName: "files",
            spec: {},
        }),
    );

    // deliver the call, copy the rows both ways, write the bucket's specification and provision it
    const looped = new Set(["sends", "replica"]);
    run(fixture.database, [
        ...space.controllers().filter((controller) => looped.has(controller.name)),
        new ConsumerController(space, [bucket], { regionId: ids.region }),
    ]);
    run(storage.database, present(kept.controllers, "the bucket service's controllers"));

    // copy the provisioned record back to the space, its files kept on the host
    const reference = buckets.reference({ bucketId });
    const copied = async () => {
        const [row] = await fixture.database
            .select({
                provider: bucket.table.provider,
                reference: bucket.table.reference,
                machineId: bucket.table.machineId,
            })
            .from(bucket.table)
            .where(eq(bucket.table.id, bucketId));

        return row;
    };
    await expect
        .poll(copied, { timeout: 10_000 })
        .toEqual({ provider: buckets.provider, reference, machineId: ids.machine });
    expect(await readdir(buckets.directory)).toEqual([bucketId]);
});

test("run the controllers each provider declares beside the buckets', sweeping the host's buckets", async () => {
    // keep a file and a blob no file references in an open bucket
    const buckets = await openBuckets();
    const bucketId = schema.identifier("bucket").parse(bucket.generateId());
    const opened = await buckets.open({ bucketId }, ids.space);
    await opened.put("notes/a.txt", "kept");
    const files = join(buckets.path({ bucketId }), "files");
    await writeFile(join(files, digestOf("abandoned")), "abandoned");

    // serve the buckets with the provider sweeping the abandoned blob
    const storage = await TestDatabase.create("sqlite", bucketDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const kept = implementBucket({
        database: storage.database,
        callKey: testCallKey,
        directory: (await openDirectory()).directory,
        providers: [bucketProvider(buckets)],
        machine: ids.machine,
        cell: ids.region,
        spaces: {
            stream: async function* () {},
            receive: () => Promise.reject(new Error("the fixture receives no changes")),
        },
    });
    run(
        storage.database,
        present(kept.controllers, "the bucket service's controllers").filter(
            (controller) => controller.name === "bucket-sweep",
        ),
    );
    await expect.poll(() => readdir(files), { timeout: 4000 }).toEqual([digestOf("kept")]);
});

/** Digest text in hex SHA-256 as the bucket names its blob. */
function digestOf(content: string): string {
    return createHash("sha256").update(content).digest("hex");
}
