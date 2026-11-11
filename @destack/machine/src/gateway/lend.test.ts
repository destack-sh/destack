import { testHost } from "@destack/test/host";
import { principal } from "@destack/access";
import { connection } from "@destack/connection";
import { TestConnection } from "@destack/connection/test";
import {
    EgressCaller,
    InstanceSpec,
    type Runtime,
    type WorkloadResource,
} from "@destack/stack/runtime";
import { ResourceId } from "@destack/resource";
import { ConnectionKind, defineConnection } from "@destack/connection/declare";
import { EGRESS_PROVIDER } from "@destack/resource";
import { Digest, Identifier, present, schema } from "@destack/schema";
import { Egress } from "@destack/object";
import { Replica, RequestId } from "@destack/sync";
import { testKeys } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { openGateway } from "./test/gateway.ts";
import { ids, fixtureBuild, TestSpace } from "@destack/stack/test";
import { RecordingTokenExchange } from "./test/exchange.ts";

/** The external API the workload calls through its connection. */
const github = defineConnection({
    name: "github",
    spec: { resource: "https://api.github.test/v3" },
});

/** The key the connection has, which only the upstream API sees. */
const KEY = "ghp_lent0123456789";

test("apply a bound connection's key to a workload's calls below its resource without showing it to the workload", async () => {
    // create a key connection of a space placed in an account's primary, bound to the notes installation, all in the space's world
    const space = await TestSpace.open();
    const world = space.simulation.environment("test");
    const { environment } = space.machine;
    const primary = await TestConnection.open({ simulation: space.simulation });
    onTestFinished(() => primary.close());
    const spaceId = schema.identifier("space").parse(Identifier.create("space", space.environment));
    const placed = await primary.place(spaceId);
    const connections = await TestConnection.open({
        primary: { ...primary.primary(spaceId), space: spaceId, enclosing: placed },
        simulation: primary.simulation,
    });
    onTestFinished(() => connections.close());

    // follow the primary's replicas up to its head, then create the connection as the owner
    connections.follow();
    const following = new AbortController();
    onTestFinished(() => following.abort());
    const head = await primary.database.log.position();
    await Replica.wait(connections.database, primary.scope.id, head, following.signal);
    const created = await connections.objects
        .mutate(connections.context().access(), {
            id: RequestId.create(environment),
            calls: [
                connection.call("create", {
                    scope: connections.scope.id,
                    ...github.provision(),
                    name: github.name,
                    protocol: { kind: "key", header: "authorization", scheme: "token" },
                }),
            ],
        })
        .then(([written]) => written);
    await connections.objects.mutate(connections.context().access(), {
        id: RequestId.create(environment),
        calls: [
            connection.call("rotate", {
                scope: connections.scope.id,
                id: created.id,
                material: { kind: "key", key: KEY },
            }),
        ],
    });
    const notes = principal.installation.reference(spaceId, ids.notes);
    await connections.bind(connections.scope.id, notes, [created.id]);
    await connections.control();

    // run an instance of the space binding the connection, addressed at its identifier
    const resource: WorkloadResource = {
        id: ResourceId.parse(created.id),
        scope: spaceId,
        kind: ConnectionKind.name,
        spec: github.spec,
        reference: created.id,
        packageId: ids.package,
        name: github.name,
        provider: EGRESS_PROVIDER,
    };
    // identify the instance by the secret its start mints
    const secrets: string[] = [];
    const spec: InstanceSpec = {
        settings: {},
        instanceId: schema
            .identifier("instance")
            .parse("instance-01996ab0-0000-7000-8000-0000000000c2"),
        installationId: ids.notes,
        name: "notes",
        scope: spaceId,
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000c3"),
        build: await fixtureBuild(ids.package, "2026.9.0", world.crypto),
        manifest: Digest.parse("d".repeat(64)),
        output: "bun",
        workload: "main",
        enabled: [],
        sandbox: {},
        directories: [],
        served: [],
        installations: [],
        resources: [resource],
    };

    // route the egress through the connection's lender to a recording upstream
    const upstream: [string, string | null, string | null][] = [];
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
        runtimes: [runtime],
        tokens: new RecordingTokenExchange(world),
        borrow: (lent, consumer) =>
            connections.as(consumer).connection.lease({ scope: lent.scope, id: lent.id }),
        send: async (request) => {
            upstream.push([
                request.url,
                request.headers.get("authorization"),
                request.headers.get("x-api-key"),
            ]);

            return Response.json({ issues: [] });
        },
    });
    const host = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: (request) => gateway.egress(request),
    });
    onTestFinished(() => host.stop(true));

    // bind the resource at start and call it over loopback as the workload
    const start = await InstanceSpec.start(spec, {
        egress: `http://127.0.0.1:${host.url.port}${Egress.path}`,
        url: `http://127.0.0.1:${host.url.port}/installation`,
        sampling: 1,
        identity: await testKeys(space.machine.environment),
        random: space.machine.environment.random,
    });
    secrets.push(start.secret);
    const binding = present(start.bindings[github.name], "the resource's binding");
    const connector = present(github.connectors[EGRESS_PROVIDER], "the connection connector");
    const client = await connector.connect(binding, github, testHost());
    const answered = await client.fetch("/repos/acme/app/issues?state=open");
    const seen = JSON.stringify([start, await answered.json(), [...answered.headers]]);

    // call it again as the installation's code in its store, through the same egress
    const stored = await gateway.send(
        { ...EgressCaller.of(spec), runs: { kind: "store" } },
        new Request(
            `http://egress.localhost${Egress.path}/${resource.reference}/repos/acme/app/issues`,
        ),
    );

    expect({
        upstream,
        status: [answered.status, stored.status],
        isKeySeen: seen.includes(KEY),
    }).toEqual({
        upstream: [
            ["https://api.github.test/v3/repos/acme/app/issues?state=open", `token ${KEY}`, null],
            ["https://api.github.test/v3/repos/acme/app/issues", `token ${KEY}`, null],
        ],
        status: [200, 200],
        isKeySeen: false,
    });
});
