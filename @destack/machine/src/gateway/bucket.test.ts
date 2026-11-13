import { testHost } from "@destack/test/host";
import { testSimulation } from "@destack/object/test";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { LocalBucket } from "@destack/bucket/bun";
import { defineBucket } from "@destack/bucket/declare";
import { S3Client, S3Server } from "@destack/bucket/s3";
import { Digest, present, schema } from "@destack/schema";
import { Egress, refusal } from "@destack/object";
import { Failure } from "@destack/error";
import { testKeys } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import type {} from "@destack/package/import-meta";
import { InstanceSpec, type Runtime, type WorkloadResource } from "../runtime/runtime.ts";
import { openGateway } from "./test/gateway.ts";
import { ids, TestSpace } from "../test/space.ts";
import { fixtureBuild } from "../test/build.ts";
import { RecordingTokenExchange } from "./test/exchange.ts";

/** The bucket the workload declares. */
const files = defineBucket({ name: "files", spec: {} }, import.meta.destack);

/** The bound bucket. */
const BUCKET_ID = schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-0000000000b1");

/** A bucket of the space the instance binds none of. */
const UNBOUND_ID = schema.identifier("bucket").parse("bucket-01996ab0-0000-7000-8000-0000000000b2");

test("connect a workload to its bound bucket over S3 at its machine's egress, and refuse a bucket it binds none of", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store the bound bucket in a local directory the machine serves over S3 as the space
    const space = await TestSpace.open();
    const directory = await world.files.makeTemporaryDirectory("destack-space-bucket-");
    onTestFinished(() => world.files.remove(directory, { recursive: true, force: true }));
    await using bucket = await LocalBucket.open(
        join(directory, BUCKET_ID),
        ids.space,
        space.environment,
    );
    const s3 = new S3Server({
        region: "auto",
        environment: space.machine.environment,
        credentials: async () => undefined,
        open: async (name, accessKeyId) =>
            name === BUCKET_ID && accessKeyId === ids.space ? bucket : undefined,
    });

    // run an instance of the space binding the bucket
    const resource: WorkloadResource = {
        id: BUCKET_ID,
        scope: ids.space,
        kind: "bucket",
        spec: {},
        reference: pathToFileURL(join(directory, BUCKET_ID)).href,
        packageId: ids.package,
        name: files.name,
        provider: "local",
    };
    // identify the instance by the secret its start mints
    const secrets: string[] = [];
    const spec: InstanceSpec = {
        settings: {},
        instanceId: schema
            .identifier("instance")
            .parse("instance-01996ab0-0000-7000-8000-0000000000b3"),
        installationId: ids.notes,
        name: "notes",
        scope: ids.space,
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000b4"),
        build: await fixtureBuild(ids.package, "2026.9.0", world.crypto),
        lock: Digest.parse("d".repeat(64)),
        output: "bun",
        workload: "main",
        enabled: [],
        sandbox: {},
        directories: [],
        served: [],
        installations: [],
        resources: [resource],
    };

    // route the instance's egress through the space's routes to the machine's S3 server
    const runtime: Runtime = {
        name: "bun",
        identify: (secret) => (secrets.includes(secret) ? spec : undefined),
        isRunning: () => true,
        start: () => Promise.reject(new Error("the fixture starts no instance")),
        stop: () => Promise.reject(new Error("the fixture stops no instance")),
        fetch: () => Promise.reject(new Error("the fixture serves no call")),
        deliver: () => Promise.reject(new Error("the fixture delivers nothing")),
        call: () => Promise.reject(new Error("the fixture calls no export")),
    };
    const gateway = openGateway(space, {
        s3,
        runtimes: [runtime],
        tokens: new RecordingTokenExchange(world),
    });
    const host = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: (request) =>
            gateway.egress(request).catch((error: unknown) => refusal(error, testHost().telemetry)),
    });
    onTestFinished(() => host.stop(true));

    // bind the bucket at start and connect over loopback as the workload
    const egress = `http://127.0.0.1:${host.url.port}${Egress.path}`;
    const start = await InstanceSpec.start(spec, {
        egress,
        url: `http://127.0.0.1:${host.url.port}/installation`,
        sampling: 1,
        identity: await testKeys(world),
        random: world.random,
    });
    secrets.push(start.secret);
    const binding = present(start.bindings[files.name], "the bucket's binding");
    const client = await S3Client.connector.connect(binding, files, testHost());
    const unbound = await S3Client.connector.connect(
        { ...binding, resource: UNBOUND_ID, reference: Egress.url(egress, UNBOUND_ID) },
        files,
        testHost(),
    );

    // put, read, list and delete files, as the bucket stores them
    const stored = await client.put("notes/first.txt", "Hello, space", {
        httpMetadata: { contentType: "text/plain" },
    });
    const head = present(await bucket.head("notes/first.txt"), "head");
    const read = present(await client.get("notes/first.txt", { range: { offset: 7 } }), "read");
    const listed = await client.list({ prefix: "notes/" });
    const listedDirectly = await bucket.list({ prefix: "notes/" });
    await client.delete("notes/first.txt");
    expect({
        binding,
        stored: [stored.version, stored.etag, stored.size, stored.httpMetadata],
        read: [read.version, read.range, await read.text()],
        listed,
        remaining: await bucket.head("notes/first.txt"),
    }).toEqual({
        binding: {
            resource: BUCKET_ID,
            kind: "bucket",
            provider: "s3",
            reference: `${egress}/${BUCKET_ID}`,
            credential: start.secret,
            scope: ids.space,
        },
        stored: [head.version, head.etag, 12, { contentType: "text/plain" }],
        read: [head.version, { offset: 7, length: 5 }, "space"],
        listed: listedDirectly,
        remaining: null,
    });

    // refuse a bucket of the space the instance binds none of
    const refused = await unbound
        .put("notes/first.txt", "Hello")
        .catch((error: unknown) => Failure.of(error));
    expect(refused).toEqual({
        code: "FORBIDDEN",
        reason: "BUCKET_DENIED",
        domain: "@destack/machine",
        message: `bucket '${UNBOUND_ID}' is forbidden: this installation does not bind it`,
    });
});
