import type { Crypto } from "@destack/environment";
import { expect, refusal, test } from "@destack/test";
import { ResourceId } from "@destack/resource";
import { ConnectionKind } from "@destack/connection/declare";
import { EGRESS_PROVIDER } from "@destack/resource";
import { PackageMount } from "@destack/object";
import { EgressCaller, type InstanceSpec, type WorkloadResource } from "@destack/stack/runtime";
import { Package, PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { Digest, schema } from "@destack/schema";
import { openGateway } from "./test/gateway.ts";
import objectPackage from "@destack/object/package";
import stackPackage from "@destack/stack/package";
import { ids, fixtureBuild, TestSpace } from "@destack/stack/test";
import accountPackage from "@destack/account/package";
import { RequestId } from "@destack/sync";

/** The package of a universe service. */
const ACCOUNT_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1");

/** A space another machine serves. */
const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f3");

/** Another package than the notes installation's. */
const OTHER_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f2");

/** An installation binding of the calling workload, with an address. */
const binding = (name: string, address: string, packageId: PackageId): WorkloadResource => ({
    id: ResourceId.parse(`installation-01996ab0-0000-7000-8000-0000000000${name}`),
    scope: ids.space,
    kind: "installation",
    spec: { package: packageId },
    reference: address,
    packageId: OTHER_PACKAGE,
    name,
    provider: "http",
});

/** An instance of the notes installation running its release's build. */
async function notesInstance(crypto: Crypto): Promise<InstanceSpec> {
    return {
        settings: {},
        instanceId: schema
            .identifier("instance")
            .parse("instance-01996ab0-0000-7000-8000-0000000000e1"),
        installationId: ids.notes,
        name: "notes",
        scope: ids.space,
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000e3"),
        build: await fixtureBuild(ids.package, "2026.9.0", crypto),
        manifest: Digest.parse("d".repeat(64)),
        output: "bun",
        workload: "main",
        enabled: [],
        sandbox: {},
        directories: [],
        served: [],
        installations: [],
        resources: [],
    };
}

test("resolve each kind of address a workload calls", async () => {
    // install the notes package in the space as its owner
    const fixture = await TestSpace.open();
    const served = await fixture.serve();
    const notesBuild = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@test/notes", version: "2026.9.0" }),
        [],
        served.environment.crypto,
    );
    const notes = await fixture.install(served, notesBuild);

    // create the acme space on another machine of the account, which publishes its endpoint
    const acmeMachine = await fixture.universe.enroll(
        fixture.owner,
        ids.account,
        "https://acme.machine.test",
    );
    await fixture.owner.client(accountPackage).space.create({
        accountId: ids.account,
        id: acme,
        requestId: RequestId.create(fixture.machine.environment),
        name: "acme",
        machine: acmeMachine.id,
    });
    await fixture.universe.simulation.idle();
    await fixture.machine.simulation.idle();

    // route with a package the space's store serves
    const gateway = openGateway(fixture, { packages: [objectPackage] });
    const peer = "board";
    const unbound = "unbound";
    const run: InstanceSpec = {
        settings: {},
        instanceId: schema
            .identifier("instance")
            .parse("instance-01996ab0-0000-7000-8000-0000000000e1"),
        installationId: notes,
        name: "notes",
        scope: ids.space,
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000e3"),
        build: await fixtureBuild(ids.package, "2026.9.0", fixture.machine.environment.crypto),
        output: "bun",
        workload: "main",
        enabled: [],
        sandbox: {},
        directories: [],
        served: [],
        installations: [
            {
                packageId: OTHER_PACKAGE,
                name: "board",
                package: OTHER_PACKAGE,
                installation: peer,
                scope: acme,
            },
        ],
        resources: [
            binding("a1", "notes", ids.package),
            binding("a3", "notes.work.acme", ids.package),
            binding("a4", "@destack/account", ACCOUNT_PACKAGE),
        ],
    };
    const resolve = async (address: string, path = "/notes/list", resources = run.resources) => {
        const resolved = gateway.resolve({ ...EgressCaller.of(run), resources }, address, path);
        const refused = await refusal(resolved);
        if (refused !== "done") {
            return refused.join(": ");
        }

        // compare an in-process route by its fetch's kind
        const route = await resolved;

        return "fetch" in route ? { ...route, fetch: typeof route.fetch } : route;
    };

    // refuse every address the workload is not bound to
    const now = fixture.machine.clock.now();
    const acmeAt = new URL((await fixture.machine.directory.locate(acme, now)).url).origin;
    const spaceAt = new URL((await fixture.machine.directory.locate(ids.space, now)).url).origin;
    const notesMount = PackageMount.path(ids.package);
    const boardMount = PackageMount.path(OTHER_PACKAGE);
    expect([
        await resolve(`notes--${ids.space}`, `${notesMount}/store/stream`, []),
        await resolve(`notes--${ids.space}`, `${notesMount}/notes/list`, []),
        await resolve(`${peer}--${acme}`, `${boardMount}/notes/list`),
        await resolve(`${peer}--${acme}`, "/notes/list"),
        await resolve(`${unbound}--${acme}`, `${boardMount}/notes/list`),
        await resolve(`${unbound}--${acme}`, `${boardMount}/store/stream`),
        await resolve(`${peer}--${ids.space}`, `${boardMount}/store/stream`),
        await resolve("@destack/object"),
        await resolve(acme, `/${ACCOUNT_PACKAGE}/accounts/list`),
        await resolve("notes"),
        await resolve("notes.work.acme"),
        await resolve("@destack/account"),
        await resolve("tasks"),
        await resolve("notes", "/notes/list", [binding("a5", "notes", OTHER_PACKAGE)]),
    ]).toEqual([
        {
            kind: "installation",
            scope: ids.space,
            installationId: notes,
            audience: ids.package,
            path: `${notesMount}/store/stream`,
        },
        "FORBIDDEN: INSTALLATION_UNBOUND: this installation does not bind installation notes",
        {
            kind: "remote",
            scope: acme,
            url: `${acmeAt}/spaces/${acme}/installations/${peer}`,
            audience: stackPackage.id,
        },
        `NOT_FOUND: PACKAGE_MOUNT_MISSING: /notes/list is not a package mount of installation ${peer}`,
        `FORBIDDEN: INSTALLATION_UNBOUND: this installation does not bind installation ${unbound}`,
        {
            kind: "remote",
            scope: acme,
            url: `${acmeAt}/spaces/${acme}/installations/${unbound}`,
            audience: stackPackage.id,
        },
        {
            kind: "remote",
            scope: ids.space,
            url: `${spaceAt}/spaces/${ids.space}/installations/${peer}`,
            audience: stackPackage.id,
        },
        { kind: "package", audience: objectPackage.id, fetch: "function" },
        {
            kind: "remote",
            scope: acme,
            url: `${acmeAt}/spaces/${acme}`,
            audience: ACCOUNT_PACKAGE,
        },
        {
            kind: "installation",
            scope: ids.space,
            installationId: notes,
            audience: ids.package,
            path: `${notesMount}/notes/list`,
        },
        `NOT_FOUND: INSTALLATION_NOT_FOUND: installation notes.work.acme of ${ids.package} not found in ${ids.space}`,
        {
            kind: "universe",
            url: `https://destack.cloud/${ACCOUNT_PACKAGE}`,
            audience: ACCOUNT_PACKAGE,
        },
        "FORBIDDEN: ADDRESS_UNBOUND: this installation does not bind tasks",
        `NOT_FOUND: INSTALLATION_NOT_FOUND: installation notes of ${OTHER_PACKAGE} not found in ${ids.space}`,
    ]);
});

test("hand an instance's telemetry to its space as the machine, stamped with its installation, instance and build, refusing an emitter it claims", async () => {
    // route with an observer recording what it receives
    const fixture = await TestSpace.open();
    const received: unknown[] = [];
    const gateway = openGateway(fixture, {
        observe: async (...exported) => {
            received.push(exported);
        },
    });

    // relay an export of the instance, then one claiming another installation
    const run = await notesInstance(fixture.machine.environment.crypto);
    const own = { key: "service.name", value: { stringValue: "notes" } };
    const forged = {
        key: "destack.installation.id",
        value: { stringValue: "installation-forged" },
    };
    await gateway.telemetry(run, "logs", {
        resourceLogs: [{ resource: { attributes: [own] }, scopeLogs: [] }],
    });
    const refused = await refusal(
        gateway.telemetry(run, "logs", {
            resourceLogs: [{ resource: { attributes: [forged, own] }, scopeLogs: [] }],
        }),
    );

    // stamp the instance's installation, instance and build, and refuse the claimed emitter
    expect({ received, refused }).toEqual({
        received: [
            [
                "logs",
                {
                    resourceLogs: [
                        {
                            resource: {
                                attributes: [
                                    own,
                                    {
                                        key: "destack.installation.id",
                                        value: { stringValue: ids.notes },
                                    },
                                    {
                                        key: "service.instance.id",
                                        value: { stringValue: run.instanceId },
                                    },
                                    {
                                        key: "destack.build.manifest",
                                        value: { stringValue: "d".repeat(64) },
                                    },
                                ],
                            },
                            scopeLogs: [],
                        },
                    ],
                },
            ],
        ],
        refused: [
            "FORBIDDEN",
            "EMITTER_ATTRIBUTE_RESERVED",
            "only the receiver can record an export's emitter, found destack.installation.id",
        ],
    });
});

/** Bind a connection to an API, addressed at its own identifier. */
function connectionResource(id: string, resource: string): WorkloadResource {
    return {
        id: ResourceId.parse(id),
        scope: ids.space,
        kind: ConnectionKind.name,
        spec: { resource, scopes: [] },
        reference: id,
        packageId: ids.package,
        name: "api",
        provider: EGRESS_PROVIDER,
    };
}

test("resolve each bound connection to its own API, borrowing it as the installation, and refuse an address nothing binds", async () => {
    // route with a lender recording what it lends
    const fixture = await TestSpace.open();
    const lent: [string, string | undefined][] = [];
    const lending = openGateway(fixture, {
        borrow: async (reference, consumer) => {
            lent.push([reference.id, consumer.id]);
            const material = { kind: "key", key: "key" } as const;
            const protocol = { kind: "key", header: "x-api-key" } as const;

            return { protocol, credential: { material }, resource: "https://unused.test" };
        },
    });

    // bind two connections to two APIs
    const first = connectionResource(
        "connection-01996ab0-0000-7000-8000-0000000000c2",
        "https://api.example.com/v1",
    );
    const second = connectionResource(
        "connection-01996ab0-0000-7000-8000-0000000000c3",
        "https://api.other.com",
    );
    const run = {
        ...(await notesInstance(fixture.machine.environment.crypto)),
        resources: [first, second],
    };
    const lend = async (address: string) => {
        const destination = await lending.resolve(EgressCaller.of(run), address, "/items");
        if (destination.kind !== "external") {
            throw new TypeError("a bound connection resolves to its external API");
        }
        const borrowed = await destination.lend(new Request(`${destination.url}/items`));

        return borrowed.request.url;
    };
    const unbound = "connection-01996ab0-0000-7000-8000-0000000000c4";

    expect({
        resolved: [await lend(first.reference), await lend(second.reference)],
        unbound: await refusal(lending.resolve(EgressCaller.of(run), unbound, "/items")),
        lent,
    }).toEqual({
        resolved: ["https://api.example.com/v1/items", "https://api.other.com/items"],
        unbound: ["FORBIDDEN", "ADDRESS_UNBOUND", `this installation does not bind ${unbound}`],
        lent: [
            [first.id, run.installationId],
            [second.id, run.installationId],
        ],
    });
});
