import { ResourceContext } from "@destack/resource/context";
import { StandardSignature } from "../trigger/index.ts";
import { MemoryBuild } from "@destack/package/test";
import { type Uplink } from "@destack/sync";
import { Clock, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineWorkload, type ServerContext } from "./workload.ts";
import { WorkloadInstance } from "./instance.ts";
import { Package } from "@destack/package";
import { PackageHandle } from "../declare/index.ts";
import { defineTrigger } from "../trigger/index.ts";
import type { RunClient } from "../trigger/index.ts";
import { hosting } from "../test/fixture.ts";
import { testCallKey } from "../test/context.ts";

/** The package-qualified repository type the calls call. */
const REPOSITORY = {
    packageId: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000001"),
    type: "repository",
};

/** The first fixture package. */
const first = PackageHandle.of(
    Package.parse({
        id: "package-01996ab0-0000-7000-8000-00000000a001",
        name: "@destack/first",
        version: "2026.9.0",
    }),
    {},
);
/** The second fixture package. */
const second = PackageHandle.of(
    Package.parse({
        id: "package-01996ab0-0000-7000-8000-00000000a002",
        name: "@destack/second",
        version: "2026.9.0",
    }),
    {},
);
/** A publisher streaming no replicas. */
const publisher = {
    stream: () => {
        throw new Error("the fixture publisher streams no replicas");
    },
    receive: () => Promise.reject(new Error("the fixture publisher receives no changes")),
} satisfies Uplink;

/** The fixture installation's server context. */
const server: ServerContext = {
    installation: "installation-01996ab0-0000-7000-8000-000000000002",
    name: "notes",
    space: "space-01996ab0-0000-7000-8000-000000000001",
    build: await MemoryBuild.write(new Map(), { package: first }),
    settings: {},
    resources: new ResourceContext(),
    keys: {
        derive: () => Promise.reject(new Error("the fixture derives no secret")),
        derivePrivateKey: () => Promise.reject(new Error("the fixture derives no private key")),
    },
    fetch: (request) => Promise.reject(new Error(`the fixture sends no request to ${request.url}`)),
    publisher,
    publisherAt: () => publisher,
    at: (address) => ({
        url: address,
        fetch: () => Promise.reject(new Error("the fixture installation cannot call an address")),
    }),
};

/** A machine the fixture workloads record no runs in. */
const runs: RunClient = {
    send: async () => {
        throw new Error("the fixture records no runs");
    },
};

/** A declared fixture webhook trigger. */
const github = defineTrigger({
    name: "github",
    on: {
        webhook: { signature: new StandardSignature(), path: "/", secret: async () => "secret" },
    },
    call: (delivery) => ({
        type: REPOSITORY,
        method: "push",
        input: { id: delivery.id },
        release: "2026.9.0",
    }),
});

test("release startup resources when the workload shuts down during startup", async () => {
    const events: string[] = [];
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });
                    context.shutdown();

                    return {
                        packages: [{ package: first, router: {} }],
                    };
                },
            }),
            {
                clock: Clock.system,
                callKey: testCallKey,
                report: (error) => {
                    throw error;
                },
                resources: hosting.resources,
                server,
                runs,
                serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
            },
        ),
    ).rejects.toMatchObject({ name: "AbortError" });
    expect(events).toEqual(["resources"]);
}, 1000);

test("start two packages and drain accepted requests before shared cleanup", async () => {
    const events: string[] = [];
    const accepted = Promise.withResolvers<void>();
    const released = Promise.withResolvers<void>();
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                context.defer(() => {
                    events.push("resources");
                });
                context.signal.addEventListener("abort", () => events.push("shutdown"));

                return {
                    packages: [
                        {
                            package: first,
                            router: {},
                            fetch: async () => {
                                accepted.resolve();
                                await released.promise;

                                return new Response("complete");
                            },
                        },
                        { package: second, router: {} },
                    ],
                };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            server,
            runs,
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 1000 }),
        },
    );

    // dispatch to every package
    const probe = await instance.fetch(second, new Request("https://fixture.test/readyz"));
    expect([probe.status, await probe.text()]).toEqual([200, "ok\n"]);
    const request = instance.fetch(first, new Request("https://fixture.test/work"));
    await accepted.promise;
    const closing = instance.close();
    expect(instance.close()).toBe(closing);
    expect(events).toEqual(["shutdown"]);

    // keep resources until the response is consumed
    released.resolve();
    const response = await request;
    expect(await response.text()).toBe("complete");
    await closing;
    expect(events).toEqual(["shutdown", "resources"]);
}, 1500);

test("retain shared resources until an overdue request observes cancellation", async () => {
    const accepted = Promise.withResolvers<void>();
    const cancelled = Promise.withResolvers<void>();
    const released = Promise.withResolvers<void>();
    const events: string[] = [];
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                context.defer(() => {
                    events.push("resources");
                });

                return {
                    packages: [
                        {
                            package: first,
                            router: {},
                            fetch: async (request: Request) => {
                                request.signal.addEventListener("abort", () => cancelled.resolve());
                                accepted.resolve();
                                await released.promise;
                                request.signal.throwIfAborted();

                                return new Response();
                            },
                        },
                    ],
                };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            server,
            runs,
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 5 }),
        },
    );

    // observe failures while the request stays open
    const response = instance.fetch(first, new Request("https://fixture.test/work"));
    const responseFailure = expect(response).rejects.toMatchObject({
        name: "TimeoutError",
        message: "server drain deadline exceeded",
    });
    await accepted.promise;
    const closing = instance.close();
    const closeFailure = expect(closing).rejects.toMatchObject({
        message: "workload shutdown failed",
        errors: [{ name: "TimeoutError", message: "server drain deadline exceeded" }],
    });
    await cancelled.promise;
    expect(events).toEqual([]);

    // release after the handler stops
    released.resolve();
    await Promise.all([responseFailure, closeFailure]);
    expect(events).toEqual(["resources"]);
}, 1500);

test("reject a package served twice and release startup resources", async () => {
    const events: string[] = [];
    const implementation = { package: first, router: {} };
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: async (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });

                    return { packages: [implementation, implementation] };
                },
            }),
            {
                clock: Clock.system,
                callKey: testCallKey,
                report: (error) => {
                    throw error;
                },
                resources: hosting.resources,
                server,
                runs,
                serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload package: ${first.name}`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("list the triggers a workload receives and find each by its package and name", async () => {
    await using instance = await WorkloadInstance.start(
        defineWorkload({ name: "fixture", start: () => ({ packages: [], triggers: [github] }) }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            server,
            runs,
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 1000 }),
        },
    );

    expect([
        instance.triggers,
        instance.trigger(github.package.id, "github"),
        instance.trigger(github.package.id, "gitlab"),
    ]).toEqual([[github], github, undefined]);
}, 1500);

test("reject a trigger registered twice and release startup resources", async () => {
    const events: string[] = [];
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });

                    return { packages: [], triggers: [github, github] };
                },
            }),
            {
                clock: Clock.system,
                callKey: testCallKey,
                report: (error) => {
                    throw error;
                },
                resources: hosting.resources,
                server,
                runs,
                serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload trigger: ${github.package.id}/github`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("give a starting workload its server context and the machine recording its runs", async () => {
    // start a workload that keeps its server context and the machine
    let received: ServerContext | undefined;
    let recording: RunClient | undefined;
    await using instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                received = context.server;
                recording = context.runs;

                return { packages: [] };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            server,
            runs,
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
        },
    );
    expect([instance.triggers, received, recording]).toEqual([[], server, runs]);
});

test("serve the next start once a reload starts it, then release the previous start's resources", async () => {
    // start a workload serving the first package, then the second after a reload
    const events: string[] = [];
    let starts = 0;
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: (context) => {
                const start = ++starts;
                context.defer(() => {
                    events.push(`release ${start}`);
                });

                return { packages: [{ package: start === 1 ? first : second, router: {} }] };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            server,
            runs,
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
        },
    );
    expect(instance.packages).toEqual([first]);

    // switch to the next start and release only the previous one
    await instance.reload();
    expect(instance.packages).toEqual([second]);
    expect(events).toEqual(["release 1"]);

    // release the next start at close
    await instance.close();
    expect(events).toEqual(["release 1", "release 2"]);
});
