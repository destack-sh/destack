import { type Publisher } from "@destack/sync";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineWorkload, type InstallationContext } from "./workload.ts";
import { WorkloadInstance } from "./instance.ts";
import { defineService } from "../declare/service.ts";
import { defineTrigger } from "../trigger/index.ts";
import type { RunClient } from "../trigger/index.ts";
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
} satisfies Publisher;

/** The fixture's installation, with publishers streaming none and a directory with no recipients. */
const installation: InstallationContext = {
    id: "installation-01996ab0-0000-7000-8000-000000000002",
    scope: "space-01996ab0-0000-7000-8000-000000000001",
    publisher,
    publisherAt: () => publisher,
    directory: {
        isHome: async () => false,
        address: async () => {},
    },
};

/** A cell the fixture workloads record no runs in. */
const runs: RunClient = {
    send: async () => {
        throw new Error("the fixture records no runs");
    },
};

/** A declared fixture webhook trigger. */
const github = defineTrigger({
    name: "github",
    on: { webhook: { verification: "github", route: "/", secret: async () => "secret" } },
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
                            route: async () => {
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
                            route: async (request: Request) => {
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

test("give a starting workload the host's audit history, its installation and the cell recording its runs", async () => {
    // start a workload that delivers one batch to the history it receives and keeps the installation and the cell
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
