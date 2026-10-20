import { ResourceContext } from "@destack/resource/context";
import { MemoryBuild } from "@destack/package/test";
import { type Uplink } from "@destack/sync";
import { Authorizer } from "@destack/access";
import { defineTable, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import type { Controller } from "../control/index.ts";
import { Health } from "../health/index.ts";
import { Server } from "../server/index.ts";
import { defineWorkload, type ServerContext } from "./workload.ts";
import { WorkloadInstance } from "./instance.ts";
import { Package } from "@destack/package";
import { PackageHandle } from "../declare/index.ts";
import { hosting } from "../test/fixture.ts";
import { testCallKey } from "../test/context.ts";
import { Clock } from "@destack/environment";

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
/** A logged table giving the controllers' database its change log. */
const marker = defineTable(
    "instance_marker",
    { id: text("id").primaryKey(), scope: text("scope").notNull() },
    { log: {} },
);

/** A publisher streaming no replicas. */
const publisher = {
    confirm: async () => {},
    stream: () => {
        throw new Error("the fixture publisher streams no replicas");
    },
    receive: () => Promise.reject(new Error("the fixture publisher receives no changes")),
    call: () => Promise.reject(new Error("the fixture publisher answers no queries")),
} satisfies Uplink;

/** The fixture installation's server context. */
const server: ServerContext = {
    installation: "installation-01996ab0-0000-7000-8000-000000000002",
    name: "notes",
    space: "space-01996ab0-0000-7000-8000-000000000001",
    build: await MemoryBuild.write(new Map(), { package: first }),
    settings: {},
    capabilities: [],
    resources: new ResourceContext(),
    keys: {
        derive: () => Promise.reject(new Error("the fixture derives no secret")),
        derivePrivateKey: () => Promise.reject(new Error("the fixture derives no private key")),
    },
    fetch: (request) => Promise.reject(new Error(`the fixture sends no request to ${request.url}`)),
    url: "https://notes--home.test",
    publisher,
    publisherAt: () => publisher,
    at: (address) => ({
        url: address,
        fetch: () => Promise.reject(new Error("the fixture installation cannot call an address")),
    }),
};

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

    // retain resources until the response is consumed
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
                serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload package: ${first.name}`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("hand the workload its webhook deliveries, refusing a workload receiving none", async () => {
    // start a workload answering deliveries with their path, and one receiving none
    const start = (webhooks: boolean) =>
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: () => ({
                    packages: [],
                    ...(webhooks
                        ? {
                              webhooks: {
                                  fetch: async (request: Request) =>
                                      new Response(new URL(request.url).pathname, { status: 202 }),
                              },
                          }
                        : {}),
                }),
            }),
            {
                clock: Clock.system,
                callKey: testCallKey,
                report: (error) => {
                    throw error;
                },
                resources: hosting.resources,
                server,
                serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 1000 }),
            },
        );
    await using receiving = await start(true);
    await using deaf = await start(false);

    // answer a delivery, and refuse one to a workload receiving none
    const answered = await receiving
        .requireWebhookHandler()
        .fetch(new Request("https://hooks.test/github/notes", { method: "POST" }));
    expect([answered.status, await answered.text()]).toEqual([202, "/github/notes"]);
    expect(() => deaf.requireWebhookHandler()).toThrow("the workload receives no webhooks");
}, 1500);

test("give a starting workload its server context", async () => {
    // start a workload that records its server context
    let received: ServerContext | undefined;
    await using instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                received = context.server;

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
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
        },
    );
    expect([instance.packages, received]).toEqual([[], server]);
});

test("serve the next start once a reload starts it, drain the previous start beside it, and release both by close", async () => {
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
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
        },
    );
    expect(instance.packages).toEqual([first]);

    // switch to the next start
    await instance.reload();
    expect(instance.packages).toEqual([second]);

    // release the previous start once drained, and the next one at close
    await instance.close();
    expect(events).toEqual(["release 1", "release 2"]);
});

test("fold the reloads requested during a start into one start after it", async () => {
    // start a workload whose second start waits until released
    let starts = 0;
    const gate = Promise.withResolvers<void>();
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async () => {
                starts += 1;
                if (starts === 2) {
                    await gate.promise;
                }

                return { packages: [{ package: first, router: {} }] };
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
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
        },
    );

    // request three reloads while the second start runs, then let it finish
    const running = instance.reload();
    const folded = [instance.reload(), instance.reload(), instance.reload()];
    gate.resolve();
    await Promise.all([running, ...folded]);

    // start once more for the three reloads
    expect(starts).toBe(3);
    await instance.close();
});

test("run one start's controllers at a time, stopping the previous start's before the next start's run", async () => {
    // record each start's controller running and stopping, over an access database
    const storage = await TestDatabase.create("sqlite", [marker], { isMigrated: true });
    onTestFinished(() => storage.close());
    const events: string[] = [];
    const running = [Promise.withResolvers<void>(), Promise.withResolvers<void>()];
    let starts = 0;
    const instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: () => {
                const start = ++starts;
                const controller: Controller = {
                    name: "follower",
                    mode: "follow",
                    list: async () => ["key"],
                    reconcile: async (_key, { signal }) => {
                        events.push(`run ${start}`);
                        running[start - 1]?.resolve();
                        await new Promise((resolve) => {
                            signal.addEventListener("abort", resolve, { once: true });
                        });
                        events.push(`stop ${start}`);

                        return undefined;
                    },
                };

                return {
                    packages: [
                        {
                            package: first,
                            router: {},
                            controllers: [controller],
                            access: {
                                authorizer: new Authorizer([], []),
                                database: storage.database,
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
            serverOptions: (handle) => ({ ...hosting, package: handle, drainTimeout: 100 }),
        },
    );

    // reload once the first start's controller runs, then wait for the second one's
    await running[0]?.promise;
    await instance.reload();
    await running[1]?.promise;
    await instance.close();

    // stop the first start's controller before the second start's runs
    expect(events).toEqual(["run 1", "stop 1", "run 2", "stop 2"]);
});

test("close a server whose controllers were never activated, running none of them", async () => {
    // start a server whose controllers wait for an activation that never comes
    const storage = await TestDatabase.create("sqlite", [marker], { isMigrated: true });
    onTestFinished(() => storage.close());
    const runs: string[] = [];
    const controller: Controller = {
        name: "never",
        watches: [],
        keys: () => [],
        list: async () => ["key"],
        reconcile: async (key) => {
            runs.push(key);

            return undefined;
        },
    };
    const started = Server.start({
        ...hosting,
        package: first,
        clock: Clock.system,
        health: new Health(first.name),
        drainTimeout: 100,
        router: {},
        controllers: [controller],
        access: { authorizer: new Authorizer([], []), database: storage.database },
        activation: new Promise<void>(() => {}),
    });

    // close it at once, as a retired start's server closes
    await started.close();
    expect(runs).toEqual([]);
});
