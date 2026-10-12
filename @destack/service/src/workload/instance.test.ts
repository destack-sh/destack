import { MemoryBuild } from "@destack/package/test";
import { type Uplink } from "@destack/sync";
import { present, Clock } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineWorkload, type InstallationContext } from "./workload.ts";
import { WorkloadInstance } from "./instance.ts";
import { defineService } from "../declare/service.ts";
import { defineTrigger } from "../trigger/index.ts";
import type { RunClient } from "../trigger/index.ts";
import { GitHubSignature } from "../github/index.ts";
import { hosting } from "../test/fixture.ts";
import { testCallKey } from "../test/context.ts";

/** The first declared fixture service. */
const first = defineService("first", {});
/** The second declared fixture service. */
const second = defineService("second", {});
/** The batches the audit history received. */
const batches: unknown[] = [];
/** An audit history keeping the batches it receives. */
const history = {
    batches,
    async ingest(batch: { readonly calls: readonly unknown[] }) {
        history.batches.push(batch);
    },
};

/** A publisher streaming no copies. */
const publisher = {
    stream: () => {
        throw new Error("the fixture publisher streams no copies");
    },
    receive: () => Promise.reject(new Error("the fixture publisher receives no changes")),
} satisfies Uplink;

/** The fixture's installation. */
const installation: InstallationContext = {
    id: "installation-01996ab0-0000-7000-8000-000000000002",
    scope: "space-01996ab0-0000-7000-8000-000000000001",
    build: await MemoryBuild.write(new Map(), { package: first.package }),
    settings: {},
    publisher,
    publisherAt: () => publisher,
    at: (address) => ({
        url: address,
        fetch: () => Promise.reject(new Error("the fixture installation reaches no address")),
    }),
    directory: {
        isHome: async () => false,
        locale: (subject) => Promise.reject(new TypeError(`the fixture knows no ${subject}`)),
        address: async () => {},
        messageKey: () => Promise.reject(new TypeError("the fixture seals no messages")),
    },
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
        webhook: { signature: new GitHubSignature(), path: "/", secret: async () => "secret" },
    },
    call: (delivery) => ({
        method: "repository.push",
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
                        services: [{ service: first, router: {} }],
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
                history,
                installation,
                runs,
                service: () => ({ ...hosting, drainTimeout: 100 }),
            },
        ),
    ).rejects.toMatchObject({ name: "AbortError" });
    expect(events).toEqual(["resources"]);
}, 1000);

test("start two services and drain accepted requests before shared cleanup", async () => {
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
                    services: [
                        {
                            service: first,
                            router: {},
                            fetch: async () => {
                                accepted.resolve();
                                await released.promise;

                                return new Response("complete");
                            },
                        },
                        { service: second, router: {} },
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
            history,
            installation,
            runs,
            service: () => ({ ...hosting, drainTimeout: 1000 }),
        },
    );

    // dispatch to every service
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
                    services: [
                        {
                            service: first,
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
            history,
            installation,
            runs,
            service: () => ({ ...hosting, drainTimeout: 5 }),
        },
    );

    // observe failures while the request stays open
    const response = instance.fetch(first, new Request("https://fixture.test/work"));
    const responseFailure = expect(response).rejects.toMatchObject({
        name: "TimeoutError",
        message: "service drain deadline exceeded",
    });
    await accepted.promise;
    const closing = instance.close();
    const closeFailure = expect(closing).rejects.toMatchObject({
        message: "workload shutdown failed",
        errors: [{ name: "TimeoutError", message: "service drain deadline exceeded" }],
    });
    await cancelled.promise;
    expect(events).toEqual([]);

    // release after the handler stops
    released.resolve();
    await Promise.all([responseFailure, closeFailure]);
    expect(events).toEqual(["resources"]);
}, 1500);

test("reject a service implemented twice and release startup resources", async () => {
    const events: string[] = [];
    const implementation = { service: first, router: {} };
    await expect(
        WorkloadInstance.start(
            defineWorkload({
                name: "fixture",
                start: async (context) => {
                    context.defer(() => {
                        events.push("resources");
                    });

                    return { services: [implementation, implementation] };
                },
            }),
            {
                clock: Clock.system,
                callKey: testCallKey,
                report: (error) => {
                    throw error;
                },
                resources: hosting.resources,
                history,
                installation,
                runs,
                service: () => ({ ...hosting, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload service: ${first.package.id}/first`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("list the triggers a workload receives and find each by its package and name", async () => {
    await using instance = await WorkloadInstance.start(
        defineWorkload({ name: "fixture", start: () => ({ services: [], triggers: [github] }) }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            history,
            installation,
            runs,
            service: () => ({ ...hosting, drainTimeout: 1000 }),
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

                    return { services: [], triggers: [github, github] };
                },
            }),
            {
                clock: Clock.system,
                callKey: testCallKey,
                report: (error) => {
                    throw error;
                },
                resources: hosting.resources,
                history,
                installation,
                runs,
                service: () => ({ ...hosting, drainTimeout: 1000 }),
            },
        ),
    ).rejects.toThrow(`duplicate workload trigger: ${github.package.id}/github`);
    expect(events).toEqual(["resources"]);
}, 1500);

test("give a starting workload the host's audit history, its installation and the machine recording its runs", async () => {
    // start a workload that delivers one batch to the history it receives and keeps the installation and the machine
    let received: InstallationContext | undefined;
    let recording: RunClient | undefined;
    await using instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                await present(context.history, "history").ingest({ calls: ["started"] });
                received = context.installation;
                recording = context.runs;

                return { services: [] };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            history,
            installation,
            runs,
            service: () => ({ ...hosting, drainTimeout: 100 }),
        },
    );
    expect([instance.triggers, history.batches.at(-1), received, recording]).toEqual([
        [],
        { calls: ["started"] },
        installation,
        runs,
    ]);
});

test("start a workload journaling only in its own database without an audit history", async () => {
    // start a workload that keeps the history it receives
    let received: unknown = "unset";
    await using _instance = await WorkloadInstance.start(
        defineWorkload({
            name: "fixture",
            start: async (context) => {
                received = context.history;

                return { services: [] };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            service: () => ({ ...hosting, drainTimeout: 100 }),
        },
    );
    expect(received).toBeUndefined();
});

test("serve the next start once a reload starts it, then release the previous start's resources", async () => {
    // start a workload serving the first service, then the second after a reload
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

                return { services: [{ service: start === 1 ? first : second, router: {} }] };
            },
        }),
        {
            clock: Clock.system,
            callKey: testCallKey,
            report: (error) => {
                throw error;
            },
            resources: hosting.resources,
            history,
            installation,
            runs,
            service: () => ({ ...hosting, drainTimeout: 100 }),
        },
    );
    expect(instance.services).toEqual([first]);

    // switch to the next start and release only the previous one
    await instance.reload();
    expect(instance.services).toEqual([second]);
    expect(events).toEqual(["release 1"]);

    // release the next start at close
    await instance.close();
    expect(events).toEqual(["release 1", "release 2"]);
});
